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

            var unmatchedGeoms = new List<string>();
            int geomTotal = 0;
            var geomsByKey = new Dictionary<string, List<StepGeom>>();
            foreach (StepGeom geom in set.geoms)
            {
                if (geom == null)
                {
                    continue;
                }
                geomTotal++;
                GameObject part = Resolve(root.transform, geom.body);
                if (part == null)
                {
                    unmatchedGeoms.Add(geom.name);
                    continue;
                }
                string geomKey = KeyOf(geom.body.indices);
                if (!geomsByKey.TryGetValue(geomKey, out List<StepGeom> geomList))
                {
                    geomList = new List<StepGeom>();
                    geomsByKey.Add(geomKey, geomList);
                }
                geomList.Add(geom);
            }
            if (unmatchedGeoms.Count > 0)
            {
                problems.Add(
                    $"{unmatchedGeoms.Count} of {geomTotal} geoms could not be placed for export: " +
                    $"{string.Join(", ", unmatchedGeoms)}");
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

            var geomNames = new Dictionary<StepGeom, string>();
            var usedGeomNames = new HashSet<string>();
            foreach (StepGeom geom in set.geoms)
            {
                if (geom != null)
                {
                    geomNames.Add(geom, Dedupe(Sanitize(geom.name), usedGeomNames, () => "geom"));
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

            bool freeRoot = set.rootMobility == StepRootMobilityMode.Free;
            string xmlPath = Path.Combine(exportDirectory, stem + ".xml");
            int bodyCount = WriteMjcf(xmlPath, stem, root.transform, freeRoot, jointsByKey, jointNames, geomsByKey, geomNames, meshIds, firstUsers, actuators);
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
            bool freeRoot,
            Dictionary<string, List<StepJoint>> jointsByKey,
            Dictionary<StepJoint, string> jointNames,
            Dictionary<string, List<StepGeom>> geomsByKey,
            Dictionary<StepGeom, string> geomNames,
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
                    GeomsByKey = geomsByKey,
                    GeomNames = geomNames,
                    MeshIds = meshIds,
                    UsedBodyNames = new HashSet<string>(),
                };
                int bodyCount = EmitBody(root, "", 0, freeRoot, context);
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
                        // Every attribute of the spec element, verbatim.
                        // The limit switches emit as "true"/"false" —
                        // explicit false disables clamping even with a
                        // range present, so the mirror is valid under
                        // autolimits. lengthrange "0 0" is the unset
                        // state and stays absent.
                        writer.WriteAttributeString("group", actuator.mj.group.ToString(CultureInfo.InvariantCulture));
                        writer.WriteAttributeString("delay", Format(actuator.mj.delay));
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
                        if (actuator.mj.lengthrangeLo != 0f || actuator.mj.lengthrangeHi != 0f)
                        {
                            writer.WriteAttributeString("lengthrange", $"{Format(actuator.mj.lengthrangeLo)} {Format(actuator.mj.lengthrangeHi)}");
                        }
                        float[] gear = actuator.mj.gear is { Length: 6 } ? actuator.mj.gear : new[] { 1f, 0f, 0f, 0f, 0f, 0f };
                        writer.WriteAttributeString(
                            "gear",
                            $"{Format(gear[0])} {Format(gear[1])} {Format(gear[2])} {Format(gear[3])} {Format(gear[4])} {Format(gear[5])}");
                        // cranklength's PRESENCE is invalid outside a
                        // slider-crank transmission (the compiler errors
                        // even at the default 0) — emit only when
                        // authored nonzero; for a joint transmission a
                        // nonzero value is an authoring mistake MuJoCo
                        // will then report loudly.
                        if (actuator.mj.cranklength != 0f)
                        {
                            writer.WriteAttributeString("cranklength", Format(actuator.mj.cranklength));
                        }
                        writer.WriteAttributeString("joint", jointNames[actuator.target]);
                        writer.WriteAttributeString("kp", Format(actuator.mj.kp));
                        // kv and dampratio are presence-exclusive — the
                        // compiler rejects both being defined even at
                        // their defaults; emit the authored one only
                        // (the form warns when both are authored).
                        if (actuator.mj.dampratio != 0f)
                        {
                            writer.WriteAttributeString("dampratio", Format(actuator.mj.dampratio));
                        }
                        else if (actuator.mj.kv != 0f)
                        {
                            writer.WriteAttributeString("kv", Format(actuator.mj.kv));
                        }
                        writer.WriteAttributeString("timeconst", Format(actuator.mj.timeconst));
                        writer.WriteAttributeString("inheritrange", Format(actuator.mj.inheritrange));
                        writer.WriteAttributeString("damping", Format(actuator.mj.damping));
                        writer.WriteAttributeString("armature", Format(actuator.mj.armature));
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

            public Dictionary<string, List<StepGeom>> GeomsByKey;

            public Dictionary<StepGeom, string> GeomNames;

            public Dictionary<Mesh, int> MeshIds;

            public HashSet<string> UsedBodyNames;
        }

        // One <geom> element from the MjGeom mirror, verbatim. pos/quat
        // are conjugated into the MJCF frame like every body-local
        // frame quantity; size, fromto, and surfacevel are spatial
        // quantities and permute component-wise; the contact-solver and
        // appearance scalars emit as authored. The mesh reference
        // attaches only for mesh-typed geoms. Baseline geoms are
        // unnamed.
        private static void WriteGeom(XmlWriter writer, string name, MjGeom mj, bool hasMesh, int meshId)
        {
            writer.WriteStartElement("geom");
            if (name != null)
            {
                writer.WriteAttributeString("name", name);
            }
            writer.WriteAttributeString("type", mj.type.ToString().ToLowerInvariant());
            writer.WriteAttributeString("pos", FormatVector(Permute(mj.pos)));
            // The orientation: the spec allows at most one mechanism and
            // its own saver always writes the canonical quat ("all frame
            // orientations are expressed as quaternions"), so the
            // authored mechanism is normalized: reconstructed as a
            // rotation in the Unity frame, conjugated into the MJCF
            // frame (M·R·M), emitted as quat. euler is interpreted with
            // MuJoCo's default eulerseq (xyz, intrinsic).
            writer.WriteAttributeString("quat", FormatQuat(Conjugated(Oriented(mj))));
            writer.WriteAttributeString("size", FormatVector(Permute(mj.size)));
            writer.WriteAttributeString("contype", mj.contype.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("conaffinity", mj.conaffinity.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("condim", mj.condim.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("group", mj.group.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("priority", mj.priority.ToString(CultureInfo.InvariantCulture));
            writer.WriteAttributeString("friction", FormatVector(mj.friction));
            writer.WriteAttributeString("solmix", Format(mj.solmix));
            writer.WriteAttributeString("solref", $"{Format(mj.solref.x)} {Format(mj.solref.y)}");
            writer.WriteAttributeString("solimp", FormatArray(mj.solimp));
            writer.WriteAttributeString("margin", Format(mj.margin));
            writer.WriteAttributeString("gap", Format(mj.gap));
            if (mj.mass != 0f)
            {
                writer.WriteAttributeString("mass", Format(mj.mass));
            }
            writer.WriteAttributeString("density", Format(mj.density));
            writer.WriteAttributeString(
                "rgba", $"{Format(mj.rgba.x)} {Format(mj.rgba.y)} {Format(mj.rgba.z)} {Format(mj.rgba.w)}");
            writer.WriteAttributeString("shellinertia", mj.shellinertia ? "true" : "false");
            if (mj.fromto is { Length: 6 })
            {
                writer.WriteAttributeString(
                    "fromto",
                    $"{Format(mj.fromto[0])} {Format(mj.fromto[2])} {Format(mj.fromto[1])} " +
                    $"{Format(mj.fromto[3])} {Format(mj.fromto[5])} {Format(mj.fromto[4])}");
            }
            writer.WriteAttributeString("fitscale", Format(mj.fitscale));
            writer.WriteAttributeString("fluidshape", mj.fluidshape.ToString().ToLowerInvariant());
            writer.WriteAttributeString("fluidcoef", FormatArray(mj.fluidcoef));
            if (mj.surfacevel is { Length: 6 })
            {
                writer.WriteAttributeString(
                    "surfacevel",
                    $"{Format(mj.surfacevel[0])} {Format(mj.surfacevel[2])} {Format(mj.surfacevel[1])} " +
                    $"{Format(mj.surfacevel[3])} {Format(mj.surfacevel[5])} {Format(mj.surfacevel[4])}");
            }
            writer.WriteAttributeString("adhesion", Format(mj.adhesion));
            if (mj.type == StepGeomType.Mesh && hasMesh)
            {
                writer.WriteAttributeString("mesh", $"mesh_{meshId:D3}");
            }
            writer.WriteEndElement();
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

            // Geom precedence: an authored StepGeom asset (persistent
            // intent) overrides the importer's StepGeomProperties
            // baseline (derived data, regenerated per reimport). The
            // attributes are the MjGeom mirror verbatim; pos/quat are
            // conjugated into the MJCF frame like every body-local
            // frame quantity, and the mesh reference follows the
            // authored type. Baseline geoms are unnamed.
            MeshFilter filter = node.GetComponent<MeshFilter>();
            int meshId = -1;
            bool hasMesh = filter != null && filter.sharedMesh != null
                && context.MeshIds.TryGetValue(filter.sharedMesh, out meshId);
            bool wroteGeom = false;
            if (context.GeomsByKey.TryGetValue(pathKey, out List<StepGeom> geoms))
            {
                foreach (StepGeom geom in geoms)
                {
                    WriteGeom(writer, context.GeomNames[geom], geom.mj, hasMesh, meshId);
                    wroteGeom = true;
                }
            }
            if (!wroteGeom && node.GetComponent<StepGeomProperties>() is StepGeomProperties baseline)
            {
                WriteGeom(writer, null, baseline.mj, hasMesh, meshId);
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

        // The authored orientation mechanism reconstructed as a Unity
        // frame rotation. Precedence follows the spec's "at most one":
        // axisangle, euler (MuJoCo's default eulerseq: xyz, intrinsic —
        // R = Rx·Ry·Rz), xyaxes, zaxis; quat is the fallback and the
        // canonical emission form (MuJoCo's own saver writes quat).
        private static Quaternion Oriented(MjGeom mj)
        {
            if (mj.axisangle is { Length: 4 })
            {
                Vector3 unityAxis = new Vector3(mj.axisangle[0], mj.axisangle[1], mj.axisangle[2]);
                return Quaternion.AngleAxis(
                    mj.axisangle[3] * Mathf.Rad2Deg, unityAxis.normalized);
            }
            if (mj.euler is { Length: 3 })
            {
                var rotation = Multiply(RotationX(mj.euler[0]), Multiply(RotationY(mj.euler[1]), RotationZ(mj.euler[2])));
                return FromConjugatedColumns(rotation);
            }
            if (mj.xyaxes is { Length: 6 })
            {
                Vector3 xAxis = new Vector3(mj.xyaxes[0], mj.xyaxes[1], mj.xyaxes[2]);
                Vector3 yAxis = new Vector3(mj.xyaxes[3], mj.xyaxes[4], mj.xyaxes[5]);
                var rotation = new double[3, 3];
                rotation[0, 0] = xAxis.x; rotation[1, 0] = xAxis.y; rotation[2, 0] = xAxis.z;
                rotation[0, 1] = yAxis.x; rotation[1, 1] = yAxis.y; rotation[2, 1] = yAxis.z;
                Vector3 zAxis = Vector3.Cross(xAxis, yAxis);
                rotation[0, 2] = zAxis.x; rotation[1, 2] = zAxis.y; rotation[2, 2] = zAxis.z;
                return FromConjugatedColumns(rotation);
            }
            if (mj.zaxis is { Length: 3 })
            {
                Vector3 direction = new Vector3(mj.zaxis[0], mj.zaxis[1], mj.zaxis[2]).normalized;
                Vector3 axis = Vector3.Cross(Vector3.forward, direction);
                float sine = axis.magnitude;
                float cosine = Vector3.Dot(Vector3.forward, direction);
                if (sine < 1e-10f)
                {
                    // Parallel (identity) or antiparallel: the minimal
                    // rotation for antiparallel is 180° about any
                    // perpendicular axis; X is deterministic.
                    return cosine > 0f ? Quaternion.identity
                        : Quaternion.AngleAxis(180f, Vector3.right);
                }
                float angle = Mathf.Atan2(sine, cosine) * Mathf.Rad2Deg;
                return Quaternion.AngleAxis(angle, axis.normalized);
            }
            return mj.quat;
        }

        // Conjugates a Unity-frame rotation expressed as orthonormal
        // columns (R' = M·R·M) and returns the quaternion.
        private static Quaternion FromConjugatedColumns(double[,] columns)
        {
            var conjugated = new double[3, 3];
            for (int i = 0; i < 3; i++)
            {
                for (int j = 0; j < 3; j++)
                {
                    conjugated[i, j] = columns[AxisMap[i], AxisMap[j]];
                }
            }
            return InertiaTensorMath.Rotation(conjugated);
        }

        private static double[,] Multiply(double[,] a, double[,] b)
        {
            var result = new double[3, 3];
            for (int i = 0; i < 3; i++)
            {
                for (int j = 0; j < 3; j++)
                {
                    double sum = 0.0;
                    for (int k = 0; k < 3; k++)
                    {
                        sum += a[i, k] * b[k, j];
                    }
                    result[i, j] = sum;
                }
            }
            return result;
        }

        private static double[,] RotationX(float radians)
        {
            double c = System.Math.Cos(radians);
            double s = System.Math.Sin(radians);
            return new double[3, 3] { { 1, 0, 0 }, { 0, c, -s }, { 0, s, c } };
        }

        private static double[,] RotationY(float radians)
        {
            double c = System.Math.Cos(radians);
            double s = System.Math.Sin(radians);
            return new double[3, 3] { { c, 0, s }, { 0, 1, 0 }, { -s, 0, c } };
        }

        private static double[,] RotationZ(float radians)
        {
            double c = System.Math.Cos(radians);
            double s = System.Math.Sin(radians);
            return new double[3, 3] { { c, -s, 0 }, { s, c, 0 }, { 0, 0, 1 } };
        }

        private static string FormatArray(float[] values)
        {
            return string.Join(" ", System.Array.ConvertAll(values, value => Format(value)));
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
