using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Xml;

using UnityEditor;
using UnityEngine;

namespace Greyhound.Step
{
    // The MJCF exporter: writes the MuJoCo model XML and the binary STL
    // mesh files directly from the imported prefab and the annotation
    // family — the Mj* payload classes are the serialization seam, so
    // the exporter writes XML elements from them without an interchange
    // format. The Rust rigid-tree exporter (M1) stays the reference
    // implementation of the schema; this writer merges the Unity-side
    // annotations into the same emitted shape.
    //
    // Frame map: MuJoCo is Z-up right-handed meters, the same frame the
    // core model projects to (identity axis map + mm→m scale). The Unity
    // hierarchy and the authored joint payloads live in Unity's projected
    // frame, so the exporter applies the inverse projection: the (x, z,
    // y) permutation for points and directions (its own inverse) and the
    // M·R·M conjugation for rotations — the same math the importer ran
    // on the way in. Vectors need no scale change: everything is meters.
    //
    // Output lives in "<step file name>~" beside the .stp — the trailing
    // tilde puts the folder on Unity's ignore list, so the artifacts
    // never enter the AssetDatabase.
    internal static class StepMjcfExporter
    {
        // mjcf axis i carries unity axis AxisMap[i]: (x, y, z)_mjcf =
        // (x, z, y)_unity.
        private static readonly int[] AxisMap = { 0, 2, 1 };

        public static void Export(StepJointSet set)
        {
            if (set == null || string.IsNullOrEmpty(set.stepAssetPath))
            {
                Debug.LogError("[StepMjcfExporter] The joint set has no STEP asset path.");
                return;
            }

            GameObject root = AssetDatabase.LoadAssetAtPath<GameObject>(set.stepAssetPath);
            if (root == null)
            {
                Debug.LogError($"{set.stepAssetPath}: no imported model found; import the STEP file first.");
                return;
            }

            var meshes = new List<Mesh>();
            var meshIds = new Dictionary<Mesh, int>();
            var firstUsers = new List<string>();
            var problems = new List<string>();
            Inspect(root.transform, meshes, meshIds, firstUsers, problems);

            var unmatched = new List<string>();
            int total = 0;
            var jointsByKey = new Dictionary<string, List<StepJoint>>();
            var movingBodies = new List<(Transform node, string pathKey)>();
            foreach (StepJoint joint in set.joints)
            {
                if (joint == null)
                {
                    continue;
                }
                total++;
                GameObject part = Resolve(root.transform, joint.body);
                if (part == null)
                {
                    unmatched.Add(joint.name);
                    continue;
                }
                string key = KeyOf(joint.body.indices);
                if (!jointsByKey.TryGetValue(key, out List<StepJoint> list))
                {
                    list = new List<StepJoint>();
                    jointsByKey.Add(key, list);
                }
                list.Add(joint);
                movingBodies.Add((part.transform, key));
            }
            if (unmatched.Count > 0)
            {
                problems.Insert(0,
                    $"{unmatched.Count} of {total} joints could not be placed for export: {string.Join(", ", unmatched)}");
            }

            // MuJoCo's own validity rule for moving bodies (its compiler
            // checks the body's own mass, else accepts a static —
            // joint-free — descendant with mass): a massless assembly
            // frame with welded meshy children is legal and emits no
            // inertial of its own.
            foreach ((Transform node, string pathKey) in movingBodies)
            {
                if (!HasValidMass(node, pathKey, jointsByKey))
                {
                    problems.Add($"moving body '{node.name}' has no mass in its welded (non-jointed) subtree");
                }
            }
            if (set.rootMobility == StepRootMobilityMode.Free && !HasValidMass(root.transform, "", jointsByKey))
            {
                problems.Add(
                    "the assembly root is free but has no mass in its welded (non-jointed) subtree; " +
                    "MuJoCo cannot compile a free body without inertia — set rootMobility to Welded or give the root mass");
            }

            var actuators = new List<StepActuator>();
            foreach (StepActuator actuator in set.actuators)
            {
                if (actuator == null)
                {
                    continue;
                }
                if (actuator.target == null)
                {
                    problems.Add($"actuator '{actuator.name}' has no target joint");
                    continue;
                }
                if (!set.joints.Contains(actuator.target))
                {
                    problems.Add($"actuator '{actuator.name}' targets joint '{actuator.target.name}' which is not in the set");
                    continue;
                }
                actuators.Add(actuator);
            }

            if (problems.Count > 0)
            {
                foreach (string problem in problems)
                {
                    Debug.LogError($"{set.stepAssetPath}: {problem}");
                }
                return;
            }

            var jointNames = new Dictionary<StepJoint, string>();
            var usedJointNames = new HashSet<string>();
            foreach (StepJoint joint in set.joints)
            {
                if (joint != null)
                {
                    jointNames.Add(joint, Dedupe(Sanitize(joint.name), usedJointNames, () => "joint"));
                }
            }

            string stepFileName = Path.GetFileName(set.stepAssetPath);
            string stem = Path.GetFileNameWithoutExtension(stepFileName);
            string exportDirectory = Path.Combine(
                Path.GetDirectoryName(Path.GetFullPath(set.stepAssetPath)), stepFileName + "~");
            string meshDirectory = Path.Combine(exportDirectory, "meshes");
            if (Directory.Exists(exportDirectory))
            {
                Directory.Delete(exportDirectory, true);
            }
            Directory.CreateDirectory(meshDirectory);

            for (int i = 0; i < meshes.Count; i++)
            {
                WriteStl(Path.Combine(meshDirectory, $"mesh_{i:D3}.stl"), meshes[i]);
            }

            string xmlPath = Path.Combine(exportDirectory, stem + ".xml");
            int bodyCount = WriteMjcf(xmlPath, stem, root.transform, set, jointsByKey, jointNames, meshIds, firstUsers, actuators);
            Debug.Log($"Exported MuJoCo model with {bodyCount} bodies and {meshes.Count} mesh assets to {xmlPath}");
        }

        // Pre-order walk over the whole hierarchy: guards every node's
        // placement (MJCF bodies have no scale; a mirrored placement is
        // the G9 gap) and collects the unique mesh assets in first-use
        // order — the same order the native walk assigns mesh indices.
        private static void Inspect(
            Transform node,
            List<Mesh> meshes,
            Dictionary<Mesh, int> meshIds,
            List<string> firstUsers,
            List<string> problems)
        {
            Vector3 scale = node.localScale;
            if (scale.x < 0f || scale.y < 0f || scale.z < 0f)
            {
                problems.Add($"'{node.name}' has a mirrored placement (negative scale); mirrored parts cannot be exported (gap G9)");
            }
            else if (Mathf.Abs(scale.x - 1f) > 1e-4f || Mathf.Abs(scale.y - 1f) > 1e-4f || Mathf.Abs(scale.z - 1f) > 1e-4f)
            {
                problems.Add($"'{node.name}' has a non-rigid placement (scale {scale}); MJCF bodies have no scale");
            }

            MeshFilter filter = node.GetComponent<MeshFilter>();
            if (filter != null && filter.sharedMesh != null && !meshIds.ContainsKey(filter.sharedMesh))
            {
                meshIds.Add(filter.sharedMesh, meshes.Count);
                meshes.Add(filter.sharedMesh);
                firstUsers.Add(node.name);
            }

            for (int i = 0; i < node.childCount; i++)
            {
                Inspect(node.GetChild(i), meshes, meshIds, firstUsers, problems);
            }
        }

        private static int WriteMjcf(
            string path,
            string stem,
            Transform root,
            StepJointSet set,
            Dictionary<string, List<StepJoint>> jointsByKey,
            Dictionary<StepJoint, string> jointNames,
            Dictionary<Mesh, int> meshIds,
            List<string> firstUsers,
            List<StepActuator> actuators)
        {
            var settings = new XmlWriterSettings
            {
                Indent = true,
                IndentChars = "  ",
                Encoding = new System.Text.UTF8Encoding(false),
            };
            using (XmlWriter writer = XmlWriter.Create(path, settings))
            {
                writer.WriteComment($" exported by com.greyhound.step from {CommentSafe(stem)} ");
                writer.WriteStartElement("mujoco");
                writer.WriteAttributeString("model", stem);

                writer.WriteStartElement("compiler");
                writer.WriteAttributeString("angle", "radian");
                writer.WriteAttributeString("meshdir", "meshes");
                writer.WriteAttributeString("autolimits", "true");
                writer.WriteAttributeString("inertiafromgeom", "auto");
                writer.WriteAttributeString("balanceinertia", "false");
                writer.WriteEndElement();

                writer.WriteStartElement("asset");
                for (int i = 0; i < firstUsers.Count; i++)
                {
                    writer.WriteComment($" first used by: {CommentSafe(firstUsers[i])} ");
                    writer.WriteStartElement("mesh");
                    writer.WriteAttributeString("name", $"mesh_{i:D3}");
                    writer.WriteAttributeString("file", $"mesh_{i:D3}.stl");
                    writer.WriteEndElement();
                }
                writer.WriteEndElement();

                writer.WriteStartElement("worldbody");
                var context = new EmissionContext
                {
                    Writer = writer,
                    JointsByKey = jointsByKey,
                    JointNames = jointNames,
                    MeshIds = meshIds,
                    UsedBodyNames = new HashSet<string>(),
                };
                int bodyCount = EmitBody(root, "", 0, set.rootMobility == StepRootMobilityMode.Free, context);
                writer.WriteEndElement();

                if (actuators.Count > 0)
                {
                    writer.WriteStartElement("actuator");
                    var usedActuatorNames = new HashSet<string>();
                    foreach (StepActuator actuator in actuators)
                    {
                        string name = Dedupe(Sanitize(actuator.name), usedActuatorNames, () => "actuator");
                        writer.WriteStartElement("position");
                        writer.WriteAttributeString("name", name);
                        writer.WriteAttributeString("joint", jointNames[actuator.target]);
                        // The spec's own limit switches, emitted verbatim:
                        // explicit ctrllimited="false" disables clamping
                        // even with a range present, so the mirror is valid
                        // under autolimits without heuristics.
                        writer.WriteAttributeString("ctrllimited", actuator.mj.ctrllimited ? "true" : "false");
                        writer.WriteAttributeString("forcelimited", actuator.mj.forcelimited ? "true" : "false");
                        if (actuator.mj.ctrllimited)
                        {
                            writer.WriteAttributeString("ctrlrange", $"{Format(actuator.mj.ctrlLo)} {Format(actuator.mj.ctrlHi)}");
                        }
                        if (actuator.mj.forcelimited)
                        {
                            writer.WriteAttributeString("forcerange", $"{Format(actuator.mj.forceLo)} {Format(actuator.mj.forceHi)}");
                        }
                        writer.WriteEndElement();
                    }
                    writer.WriteEndElement();
                }

                writer.WriteEndElement();
                writer.WriteEndDocument();
                return bodyCount;
            }
        }

        private sealed class EmissionContext
        {
            public XmlWriter Writer;

            public Dictionary<string, List<StepJoint>> JointsByKey;

            public Dictionary<StepJoint, string> JointNames;

            public Dictionary<Mesh, int> MeshIds;

            public HashSet<string> UsedBodyNames;
        }

        private static int EmitBody(Transform node, string pathKey, int nodeIndex, bool isRoot, EmissionContext context)
        {
            XmlWriter writer = context.Writer;
            string rawName = node.name;
            writer.WriteComment($" part: {CommentSafe(rawName)} ");
            writer.WriteStartElement("body");
            writer.WriteAttributeString("name", Dedupe(Sanitize(rawName), context.UsedBodyNames, () => $"part{nodeIndex}"));
            writer.WriteAttributeString("pos", FormatVector(Permute(node.localPosition)));
            writer.WriteAttributeString("quat", FormatQuat(Conjugated(node.localRotation)));

            if (isRoot)
            {
                writer.WriteStartElement("freejoint");
                writer.WriteEndElement();
            }

            if (context.JointsByKey.TryGetValue(pathKey, out List<StepJoint> joints))
            {
                foreach (StepJoint joint in joints)
                {
                    writer.WriteStartElement("joint");
                    writer.WriteAttributeString("name", context.JointNames[joint]);
                    writer.WriteAttributeString("type", JointTypeString(joint.mj.type));
                    writer.WriteAttributeString("pos", FormatVector(Permute(joint.mj.pos)));
                    writer.WriteAttributeString("axis", FormatVector(Permute(joint.mj.axis)));
                    // The spec's own limit switch, emitted verbatim like
                    // the actuator's: explicit false disables the limit
                    // even with a range present.
                    writer.WriteAttributeString("limited", joint.mj.limited ? "true" : "false");
                    if (joint.mj.limited)
                    {
                        writer.WriteAttributeString("range", $"{Format(joint.mj.rangeLo)} {Format(joint.mj.rangeHi)}");
                    }
                    writer.WriteAttributeString("damping", Format(joint.mj.damping));
                    writer.WriteAttributeString("frictionloss", Format(joint.mj.frictionloss));
                    writer.WriteAttributeString("armature", Format(joint.mj.armature));
                    writer.WriteEndElement();
                }
            }

            MeshFilter filter = node.GetComponent<MeshFilter>();
            if (filter != null && filter.sharedMesh != null && context.MeshIds.TryGetValue(filter.sharedMesh, out int meshId))
            {
                writer.WriteStartElement("geom");
                writer.WriteAttributeString("type", "mesh");
                writer.WriteAttributeString("mesh", $"mesh_{meshId:D3}");
                writer.WriteEndElement();
            }

            StepMassProperties properties = node.GetComponent<StepMassProperties>();
            if (properties != null)
            {
                writer.WriteStartElement("inertial");
                writer.WriteAttributeString("pos", FormatVector(Permute(properties.centreOfMass)));
                writer.WriteAttributeString("quat", FormatQuat(Conjugated(properties.inertiaTensorRotation)));
                writer.WriteAttributeString("mass", Format(properties.massKilograms));
                writer.WriteAttributeString(
                    "diaginertia",
                    $"{Format(properties.inertiaTensor.x)} {Format(properties.inertiaTensor.y)} {Format(properties.inertiaTensor.z)}");
                writer.WriteEndElement();
            }

            int bodyCount = 1;
            for (int i = 0; i < node.childCount; i++)
            {
                Transform child = node.GetChild(i);
                string childKey = pathKey.Length == 0 ? i.ToString() : $"{pathKey}/{i}";
                bodyCount += EmitBody(child, childKey, nodeIndex + bodyCount, false, context);
            }

            writer.WriteEndElement();
            return bodyCount;
        }

        // Binary STL from the Unity mesh asset: vertices are body-local
        // meters in the Unity frame, so the export is the permutation
        // alone. The import flipped two indices per triangle for Unity's
        // winding; flipping again recovers the right-handed order the
        // STL stores, and the facet normal follows that winding. MuJoCo
        // recenters meshes itself, so no geom pose is needed.
        private static void WriteStl(string path, Mesh mesh)
        {
            Vector3[] vertices = mesh.vertices;
            int[] triangles = mesh.triangles;
            using (FileStream stream = File.Create(path))
            using (var writer = new System.IO.BinaryWriter(stream))
            {
                writer.Write(new byte[80]);
                writer.Write(triangles.Length / 3);
                for (int t = 0; t < triangles.Length; t += 3)
                {
                    Vector3 a = Permute(vertices[triangles[t]]);
                    Vector3 b = Permute(vertices[triangles[t + 2]]);
                    Vector3 c = Permute(vertices[triangles[t + 1]]);
                    Vector3 normal = Vector3.Cross(b - a, c - a);
                    normal = normal.sqrMagnitude > 0f ? normal.normalized : Vector3.zero;
                    WriteFloat3(writer, normal);
                    WriteFloat3(writer, a);
                    WriteFloat3(writer, b);
                    WriteFloat3(writer, c);
                    writer.Write((ushort)0);
                }
            }
        }

        private static void WriteFloat3(System.IO.BinaryWriter writer, Vector3 value)
        {
            writer.Write(value.x);
            writer.Write(value.y);
            writer.Write(value.z);
        }

        private static Vector3 Permute(Vector3 unity)
        {
            return new Vector3(unity.x, unity.z, unity.y);
        }

        // The Unity rotation re-expressed in the MJCF frame: R' = M·R·M
        // with M the (x, z, y) permutation. The result comes back as a
        // Unity quaternion (x, y, z, w); MJCF wants w first.
        private static Quaternion Conjugated(Quaternion rotation)
        {
            var matrix = new double[3, 3];
            RotationMatrix(rotation, matrix);
            var conjugated = new double[3, 3];
            for (int i = 0; i < 3; i++)
            {
                for (int j = 0; j < 3; j++)
                {
                    conjugated[i, j] = matrix[AxisMap[i], AxisMap[j]];
                }
            }
            return InertiaTensorMath.Rotation(conjugated);
        }

        // Rotation matrix of a Unity quaternion (v_parent = q * v_local),
        // same convention as InertiaTensorMath.
        private static void RotationMatrix(Quaternion q, double[,] r)
        {
            double x = q.x;
            double y = q.y;
            double z = q.z;
            double w = q.w;
            r[0, 0] = 1.0 - 2.0 * (y * y + z * z);
            r[0, 1] = 2.0 * (x * y - w * z);
            r[0, 2] = 2.0 * (x * z + w * y);
            r[1, 0] = 2.0 * (x * y + w * z);
            r[1, 1] = 1.0 - 2.0 * (x * x + z * z);
            r[1, 2] = 2.0 * (y * z - w * x);
            r[2, 0] = 2.0 * (x * z - w * y);
            r[2, 1] = 2.0 * (y * z + w * x);
            r[2, 2] = 1.0 - 2.0 * (x * x + y * y);
        }

        // MuJoCo's moving-body validity rule, transcribed from its
        // compiler's CheckBodyMassInertia: the body's own mass counts, or
        // the first static (joint-free) descendant with mass. The joint
        // set decides which descendants are static, so the check is the
        // exporter's, not the hierarchy's.
        private static bool HasValidMass(
            Transform node, string pathKey, Dictionary<string, List<StepJoint>> jointsByKey)
        {
            StepMassProperties properties = node.GetComponent<StepMassProperties>();
            if (properties != null && properties.massKilograms > 0.0)
            {
                return true;
            }

            for (int i = 0; i < node.childCount; i++)
            {
                Transform child = node.GetChild(i);
                string childKey = pathKey.Length == 0 ? i.ToString() : $"{pathKey}/{i}";
                if (jointsByKey.ContainsKey(childKey))
                {
                    continue;
                }
                if (HasValidMass(child, childKey, jointsByKey))
                {
                    return true;
                }
            }
            return false;
        }

        private static GameObject Resolve(Transform root, StepSiblingPath path)
        {
            if (path == null || path.indices == null)
            {
                return null;
            }
            Transform node = root;
            foreach (int index in path.indices)
            {
                if (node == null || index < 0 || index >= node.childCount)
                {
                    return null;
                }
                node = node.GetChild(index);
            }
            return node.gameObject;
        }

        private static string KeyOf(int[] indices)
        {
            return indices == null ? "" : string.Join("/", indices);
        }

        private static string JointTypeString(StepJointType type)
        {
            switch (type)
            {
                case StepJointType.Slide:
                    return "slide";
                case StepJointType.Hinge:
                    return "hinge";
                default:
                    return type.ToString().ToLowerInvariant();
            }
        }

        // MJCF names are free strings; the character restriction is
        // exporter policy for predictable, comment-safe names (the M1
        // naming policy).
        private static string Sanitize(string name)
        {
            var builder = new StringBuilder(name.Length);
            foreach (char character in name)
            {
                bool allowed = (character >= 'a' && character <= 'z')
                    || (character >= 'A' && character <= 'Z')
                    || (character >= '0' && character <= '9')
                    || character == '_' || character == '.' || character == '-';
                builder.Append(allowed ? character : '_');
            }
            return builder.ToString();
        }

        private static string Dedupe(string name, HashSet<string> used, System.Func<string> fallback)
        {
            if (name.Length == 0)
            {
                name = fallback();
            }
            string candidate = name;
            for (int suffix = 2; used.Contains(candidate); suffix++)
            {
                candidate = $"{name}_{suffix}";
            }
            used.Add(candidate);
            return candidate;
        }

        private static string CommentSafe(string text)
        {
            return text.Replace("--", "__");
        }

        private static string Format(double value)
        {
            return value.ToString("R", CultureInfo.InvariantCulture);
        }

        private static string Format(float value)
        {
            return value.ToString("R", CultureInfo.InvariantCulture);
        }

        private static string FormatVector(Vector3 value)
        {
            return $"{Format(value.x)} {Format(value.y)} {Format(value.z)}";
        }

        private static string FormatQuat(Quaternion value)
        {
            return $"{Format(value.w)} {Format(value.x)} {Format(value.y)} {Format(value.z)}";
        }
    }
}
