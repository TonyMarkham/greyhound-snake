using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Xml;

#if GREYHOUND_STEP
using Greyhound.Step;
#endif

using UnityEditor;
using UnityEngine;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The MJCF exporter: writes the MuJoCo model XML and binary STL mesh
    // files for one Root, into a folder the caller chooses. Ported from
    // the step package's StepMjcfExporter; the step package is not used
    // except through the optional GREYHOUND_STEP converters.
    //
    // What is written, per part of Root.root (the model asset), as one
    // <body> each, in hierarchy order:
    //   - <freejoint> on the model root when rootMobility is Free.
    //   - <joint>, <site> for the Joint and Site assets whose body names
    //     this part.
    //   - <geom>: the part's default geom (with the step package: the
    //     importer's StepGeomProperties, converted in memory), plus every
    //     Geom asset on the part. A Geom asset with overrideDefault, or of
    //     type Null, drops the default; every Geom asset whose type is not
    //     Null is written.
    //     A MeshFilter mesh is written as an STL only if its default geom
    //     is.
    //   - <inertial>: with the step package, the importer's
    //     StepMassProperties, converted in memory, unless a Geom asset
    //     overrides the GameObject (then its mass and inertia come from
    //     the written geoms). Otherwise none, and
    //     MuJoCo derives mass and inertia from the body's geoms
    //     (compiler inertiafromgeom="auto").
    // then <actuator> and <sensor> sections from the Actuator and Sensor
    // assets.
    //
    // Attribute policy: an attribute is written only when its value
    // differs from the schema default (mjcf-3.15.0.schema), so the file
    // states what was authored and MuJoCo supplies the rest. Exceptions:
    // option timestep and integrator are always written, so the file
    // states its cadence; a velocity actuator's kv is always written,
    // because the field default (0) is not the element's (1).
    //
    // Frames: everything authored is in Unity axes. Points and
    // directions are written as (x, z, y); rotations as the conjugated
    // quaternion (see MjcfQuat). Units need no change: meters, kg.
    internal static class MjcfExporter
    {
        // Returns every file written, XML first, or null when the export
        // was refused (each reason is logged as an error).
        public static List<string> Export(SO.Root root, string folder)
        {
            var problems = new List<string>();
            if (root == null)
            {
                Debug.LogError("[MjcfExporter] No Root to export.");
                return null;
            }
            GameObject model = root.root;
            if (model == null)
            {
                Debug.LogError($"{root.name}: Root has no model; set its root field to the model asset.", root);
                return null;
            }

            Inspect(model.transform, problems);

            Dictionary<string, List<SO.Joint>> jointsByKey =
                Place(root.joints, joint => joint.body, "joint", model, problems);
            Dictionary<string, List<SO.Site>> sitesByKey =
                Place(root.sites, site => site.body, "site", model, problems);
            Dictionary<string, List<SO.Geom>> geomsByKey =
                Place(root.geoms, geom => geom.body, "geom", model, problems);

            var meshes = new List<Mesh>();
            var meshIds = new Dictionary<Mesh, int>();
            var firstUsers = new List<string>();
            CollectMeshes(model.transform, "", geomsByKey, meshes, meshIds, firstUsers);

            List<SO.Actuator> actuators = CheckActuators(root, problems);
            List<SO.Sensor> sensors = CheckSensors(root, model, problems);

            // MuJoCo rejects a moving body without mass (its compiler's
            // CheckBodyMassInertia). Checked only where mass is known
            // before compiling: a converted inertial, or a geom that
            // contributes mass.
            foreach (KeyValuePair<string, List<SO.Joint>> entry in jointsByKey)
            {
                Transform node = Resolve(model.transform, entry.Value[0].body);
                if (!HasValidMass(node, entry.Key, jointsByKey, geomsByKey))
                {
                    problems.Add($"moving body '{node.name}' has no mass in its welded (non-jointed) subtree");
                }
            }
            if (root.rootMobility == RootMobilityMode.Free
                && !HasValidMass(model.transform, "", jointsByKey, geomsByKey))
            {
                problems.Add(
                    "the model root is free but has no mass in its welded (non-jointed) subtree; " +
                    "set rootMobility to Welded or give the root mass");
            }

            // NaN fails every comparison, so it is caught by the same test.
            if (!(root.timestepSeconds > 0f) || float.IsInfinity(root.timestepSeconds))
            {
                problems.Add($"timestep is {Format(root.timestepSeconds)} s; it must be a positive number of seconds");
            }
            if (IntegratorKeyword(root.integrator) == null)
            {
                problems.Add($"integrator is {(int)root.integrator}, which is not an Integrator value");
            }

            if (problems.Count > 0)
            {
                foreach (string problem in problems)
                {
                    Debug.LogError($"{root.name}: {problem}", root);
                }
                Debug.LogError($"{root.name}: export refused ({problems.Count} problem(s)); nothing was written.", root);
                return null;
            }

            var names = new Names(root);
            string stem = Sanitize(model.name);
            if (stem.Length == 0)
            {
                stem = "model";
            }
            string meshDirectory = Path.Combine(folder, "meshes");
            Directory.CreateDirectory(meshDirectory);

            var written = new List<string>();
            string xmlPath = Path.Combine(folder, stem + ".xml");
            written.Add(xmlPath);
            for (int i = 0; i < meshes.Count; i++)
            {
                string meshPath = Path.Combine(meshDirectory, MeshName(i) + ".stl");
                WriteStl(meshPath, meshes[i]);
                written.Add(meshPath);
            }

            int bodyCount = WriteMjcf(
                xmlPath, root, model, jointsByKey, sitesByKey, geomsByKey, names, meshIds, firstUsers, actuators, sensors);

            var report = new StringBuilder();
            report.AppendLine(
                $"{root.name}: exported {bodyCount} bodies and {meshes.Count} meshes to {Path.GetFullPath(folder)}");
            foreach (string path in written)
            {
                report.AppendLine("  " + Path.GetFullPath(path));
            }
            Debug.Log(report.ToString(), root);
            return written;
        }

        // ------------------------------------------------------------ checks

        // Pre-order walk over the model: guards every node's placement
        // (MJCF bodies have no scale).
        private static void Inspect(Transform node, List<string> problems)
        {
            Vector3 scale = node.localScale;
            if (scale.x < 0f || scale.y < 0f || scale.z < 0f)
            {
                problems.Add($"'{node.name}' has a mirrored placement (negative scale); mirrored parts cannot be exported");
            }
            else if (Mathf.Abs(scale.x - 1f) > 1e-4f || Mathf.Abs(scale.y - 1f) > 1e-4f || Mathf.Abs(scale.z - 1f) > 1e-4f)
            {
                problems.Add($"'{node.name}' has a non-rigid placement (scale {scale}); MJCF bodies have no scale");
            }

            for (int i = 0; i < node.childCount; i++)
            {
                Inspect(node.GetChild(i), problems);
            }
        }

        // Pre-order walk collecting the unique MeshFilter meshes that a
        // written geom uses, in first-use order. A GameObject's MeshFilter
        // mesh is used only by its default geom, so a mesh is left out
        // (no STL, no <asset> entry) when the default geom is overridden
        // or the GameObject has none.
        private static void CollectMeshes(
            Transform node,
            string pathKey,
            Dictionary<string, List<SO.Geom>> geomsByKey,
            List<Mesh> meshes,
            Dictionary<Mesh, int> meshIds,
            List<string> firstUsers)
        {
            MeshFilter filter = node.GetComponent<MeshFilter>();
            bool used = GeomsOf(node, pathKey, geomsByKey)
                .Any(entry => entry.asset == null && entry.geom.type == GeomType.Mesh);
            if (used && filter != null && filter.sharedMesh != null && !meshIds.ContainsKey(filter.sharedMesh))
            {
                meshIds.Add(filter.sharedMesh, meshes.Count);
                meshes.Add(filter.sharedMesh);
                firstUsers.Add(node.name);
            }

            for (int i = 0; i < node.childCount; i++)
            {
                CollectMeshes(node.GetChild(i), ChildKey(pathKey, i), geomsByKey, meshes, meshIds, firstUsers);
            }
        }

        // Groups the elements by the part they sit on (key = the index
        // path, e.g. "2/0"). An element is refused when its body names a
        // different model than Root.root, or its path no longer resolves.
        private static Dictionary<string, List<T>> Place<T>(
            List<T> elements, Func<T, SiblingPath> bodyOf, string kind, GameObject model, List<string> problems)
            where T : UnityEngine.Object
        {
            var byKey = new Dictionary<string, List<T>>();
            if (elements == null)
            {
                return byKey;
            }
            foreach (T element in elements)
            {
                if (element == null)
                {
                    continue;
                }
                SiblingPath body = bodyOf(element);
                if (body == null || body.root == null || !HasTarget(body))
                {
                    problems.Add($"{kind} '{element.name}' has no target part (drop one on its body field)");
                    continue;
                }
                if (body.root != model)
                {
                    problems.Add($"{kind} '{element.name}' is on model '{body.root.name}', not this Root's model '{model.name}'");
                    continue;
                }
                if (Resolve(model.transform, body) == null)
                {
                    problems.Add($"{kind} '{element.name}' no longer resolves: path [{KeyOf(body.indices)}] is not in '{model.name}'");
                    continue;
                }
                string key = KeyOf(body.indices);
                if (!byKey.TryGetValue(key, out List<T> list))
                {
                    list = new List<T>();
                    byKey.Add(key, list);
                }
                list.Add(element);
            }
            return byKey;
        }

        // MJCF needs exactly one transmission per actuator: a joint
        // (target or jointinparent), a site (refsite optional), or a
        // slider-crank (cranksite with slidersite). Every referenced joint
        // or site must be in this Root, or its name is not in the file.
        private static List<SO.Actuator> CheckActuators(SO.Root root, List<string> problems)
        {
            var valid = new List<SO.Actuator>();
            if (root.actuators == null)
            {
                return valid;
            }
            foreach (SO.Actuator actuator in root.actuators)
            {
                if (actuator == null)
                {
                    continue;
                }
                string label = $"actuator '{actuator.name}'";
                int transmissions = (actuator.target != null ? 1 : 0)
                    + (actuator.jointinparent != null ? 1 : 0)
                    + (actuator.site != null ? 1 : 0)
                    + (actuator.cranksite != null ? 1 : 0);
                if (transmissions != 1)
                {
                    problems.Add($"{label} needs exactly one transmission (target, jointinparent, site or cranksite); it has {transmissions}");
                    continue;
                }
                if (actuator.refsite != null && actuator.site == null)
                {
                    problems.Add($"{label} has a refsite but no site; refsite only applies to a site transmission");
                    continue;
                }
                if ((actuator.cranksite != null) != (actuator.slidersite != null))
                {
                    problems.Add($"{label}: a slider-crank needs both cranksite and slidersite");
                    continue;
                }
                bool known = KnownJoint(root, actuator.target, label, "target", problems)
                    & KnownJoint(root, actuator.jointinparent, label, "jointinparent", problems)
                    & KnownSite(root, actuator.site, label, "site", problems)
                    & KnownSite(root, actuator.refsite, label, "refsite", problems)
                    & KnownSite(root, actuator.slidersite, label, "slidersite", problems)
                    & KnownSite(root, actuator.cranksite, label, "cranksite", problems);
                if (!known)
                {
                    continue;
                }
                if (ActuatorElement(actuator.actuator.type) == null)
                {
                    problems.Add($"{label} has type {(int)actuator.actuator.type}, which is not an ActuatorType value");
                    continue;
                }
                valid.Add(actuator);
            }
            return valid;
        }

        // objtype/objname are required; reftype/refname are optional and
        // written together (an empty refname* field means world). Each
        // named object must be written to the file: a Site in this Root,
        // a Geom in this Root that is not of type Null, or a GameObject of
        // this Root's model (Body, XBody). Camera has no counterpart.
        private static List<SO.Sensor> CheckSensors(SO.Root root, GameObject model, List<string> problems)
        {
            var valid = new List<SO.Sensor>();
            if (root.sensors == null)
            {
                return valid;
            }
            foreach (SO.Sensor sensor in root.sensors)
            {
                if (sensor == null)
                {
                    continue;
                }
                string label = $"sensor '{sensor.name}'";
                bool objOk = CheckFrame(
                    root, model, sensor.objtype, sensor.objnameSite, sensor.objnameGeom, sensor.objnameBody,
                    true, label, "objname", problems);
                bool refOk = CheckFrame(
                    root, model, sensor.reftype, sensor.refnameSite, sensor.refnameGeom, sensor.refnameBody,
                    false, label, "refname", problems);
                if (objOk && refOk)
                {
                    valid.Add(sensor);
                }
            }
            return valid;
        }

        private static bool CheckFrame(
            SO.Root root,
            GameObject model,
            FrameObject type,
            SO.Site site,
            SO.Geom geom,
            SiblingPath body,
            bool required,
            string label,
            string field,
            List<string> problems)
        {
            switch (type)
            {
                case FrameObject.Site:
                    if (site == null)
                    {
                        break;
                    }
                    return KnownSite(root, site, label, field, problems);
                case FrameObject.Geom:
                    if (geom == null)
                    {
                        break;
                    }
                    if (root.geoms == null || !root.geoms.Contains(geom))
                    {
                        problems.Add($"{label}: {field} geom '{geom.name}' is not in this Root's geoms");
                        return false;
                    }
                    if (geom.geom == null || geom.geom.type == GeomType.Null)
                    {
                        problems.Add($"{label}: {field} geom '{geom.name}' is of type Null, so it is not written");
                        return false;
                    }
                    return true;
                case FrameObject.Body:
                case FrameObject.XBody:
                    if (body == null || body.root == null || !HasTarget(body))
                    {
                        break;
                    }
                    if (body.root != model || Resolve(model.transform, body) == null)
                    {
                        problems.Add($"{label}: {field} body is not a GameObject of this Root's model '{model.name}'");
                        return false;
                    }
                    return true;
                case FrameObject.Camera:
                    problems.Add($"{label}: {field} type Camera is not supported (the package has no camera type)");
                    return false;
                default:
                    problems.Add($"{label}: {field} type {(int)type} is not a FrameObject value");
                    return false;
            }
            if (required)
            {
                problems.Add($"{label} has no {field} ({type})");
                return false;
            }
            return true;
        }

        private static bool KnownJoint(SO.Root root, SO.Joint joint, string label, string field, List<string> problems)
        {
            if (joint == null || (root.joints != null && root.joints.Contains(joint)))
            {
                return true;
            }
            problems.Add($"{label}: {field} joint '{joint.name}' is not in this Root's joints");
            return false;
        }

        private static bool KnownSite(SO.Root root, SO.Site site, string label, string field, List<string> problems)
        {
            if (site == null || (root.sites != null && root.sites.Contains(site)))
            {
                return true;
            }
            problems.Add($"{label}: {field} site '{site.name}' is not in this Root's sites");
            return false;
        }

        // The body's own mass counts, or the first static (joint-free)
        // descendant with mass. Mass known before compiling: a converted
        // inertial with mass > 0, or an exported geom that contributes
        // mass (mass > 0, or density > 0 with no mass, in a group inside
        // the compiler's default inertiagrouprange 0..5).
        private static bool HasValidMass(
            Transform node,
            string pathKey,
            Dictionary<string, List<SO.Joint>> jointsByKey,
            Dictionary<string, List<SO.Geom>> geomsByKey)
        {
            Inertial inertial = DefaultInertial(node, pathKey, geomsByKey);
            if (inertial != null && inertial.mass > 0f)
            {
                return true;
            }
            foreach (Geom geom in GeomsOf(node, pathKey, geomsByKey).Select(entry => entry.geom))
            {
                bool inGroup = geom.group >= 0 && geom.group <= 5;
                if (inGroup && (geom.mass > 0f || (geom.mass == 0f && geom.density > 0f)))
                {
                    return true;
                }
            }
            for (int i = 0; i < node.childCount; i++)
            {
                string childKey = ChildKey(pathKey, i);
                if (jointsByKey.ContainsKey(childKey))
                {
                    continue;
                }
                if (HasValidMass(node.GetChild(i), childKey, jointsByKey, geomsByKey))
                {
                    return true;
                }
            }
            return false;
        }

        // ------------------------------------------------- default sources

        // The part's default geom, or null. With the step package, the
        // importer's StepGeomProperties; the mesh name is filled at write
        // time.
        private static Geom DefaultGeom(Transform node)
        {
#if GREYHOUND_STEP
            if (node.TryGetComponent(out StepGeomProperties properties))
            {
                return StepGeomConverter.ToGeom(properties);
            }
#endif
            return null;
        }

        // The GameObject's default inertial, or null. With the step package,
        // the importer's StepMassProperties. A Geom asset that
        // overrides (Overrides: overrideDefault, or type Null) also overrides
        // the mass and inertia: no <inertial> is written, and MuJoCo derives
        // them from the written geoms (inertiafromgeom auto); for a Null
        // geom that is none.
        private static Inertial DefaultInertial(
            Transform node, string pathKey, Dictionary<string, List<SO.Geom>> geomsByKey)
        {
            if (geomsByKey.TryGetValue(pathKey, out List<SO.Geom> assets) && assets.Any(Overrides))
            {
                return null;
            }
#if GREYHOUND_STEP
            if (node.TryGetComponent(out StepMassProperties properties))
            {
                return StepInertialConverter.ToInertial(properties);
            }
#endif
            return null;
        }

        // Whether a Geom asset overrides its GameObject's default mesh geom
        // and default <inertial>: overrideDefault on, or type Null (a Null
        // geom always overrides; with no shape it has no reliable inertia).
        private static bool Overrides(SO.Geom asset)
        {
            return asset.overrideDefault || asset.geom == null || asset.geom.type == GeomType.Null;
        }

        // The geoms a part exports, in order: the default (unless a Geom
        // asset on the part overrides it), then every Geom asset whose type
        // is not Null. A Null Geom asset writes nothing and always
        // overrides. The asset is null for the default geom.
        private static List<(Geom geom, SO.Geom asset)> GeomsOf(
            Transform node, string pathKey, Dictionary<string, List<SO.Geom>> geomsByKey)
        {
            var result = new List<(Geom, SO.Geom)>();
            geomsByKey.TryGetValue(pathKey, out List<SO.Geom> assets);
            bool overridden = assets != null && assets.Any(Overrides);
            Geom fallback = overridden ? null : DefaultGeom(node);
            if (fallback != null)
            {
                result.Add((fallback, null));
            }
            if (assets != null)
            {
                foreach (SO.Geom asset in assets)
                {
                    if (asset.geom != null && asset.geom.type != GeomType.Null)
                    {
                        result.Add((asset.geom, asset));
                    }
                }
            }
            return result;
        }

        // ------------------------------------------------------------ write

        private static int WriteMjcf(
            string path,
            SO.Root root,
            GameObject model,
            Dictionary<string, List<SO.Joint>> jointsByKey,
            Dictionary<string, List<SO.Site>> sitesByKey,
            Dictionary<string, List<SO.Geom>> geomsByKey,
            Names names,
            Dictionary<Mesh, int> meshIds,
            List<string> firstUsers,
            List<SO.Actuator> actuators,
            List<SO.Sensor> sensors)
        {
            var settings = new XmlWriterSettings
            {
                Indent = true,
                IndentChars = "  ",
                Encoding = new UTF8Encoding(false),
            };
            using (XmlWriter writer = XmlWriter.Create(path, settings))
            {
                var w = new Attributes(writer);
                writer.WriteComment($" exported by com.greyhound.mujoco from {CommentSafe(root.name)} ({CommentSafe(model.name)}) ");
                writer.WriteStartElement("mujoco");
                writer.WriteAttributeString("model", Sanitize(model.name));

                writer.WriteStartElement("compiler");
                writer.WriteAttributeString("angle", "radian");
                writer.WriteAttributeString("meshdir", "meshes");
                writer.WriteAttributeString("autolimits", "true");
                writer.WriteAttributeString("inertiafromgeom", "auto");
                writer.WriteAttributeString("balanceinertia", "false");
                writer.WriteEndElement();

                WriteOption(writer, w, root);

                writer.WriteStartElement("asset");
                for (int i = 0; i < firstUsers.Count; i++)
                {
                    writer.WriteComment($" first used by: {CommentSafe(firstUsers[i])} ");
                    writer.WriteStartElement("mesh");
                    writer.WriteAttributeString("name", MeshName(i));
                    writer.WriteAttributeString("file", MeshName(i) + ".stl");
                    writer.WriteEndElement();
                }
                writer.WriteEndElement();

                writer.WriteStartElement("worldbody");
                var context = new Context
                {
                    Writer = writer,
                    W = w,
                    JointsByKey = jointsByKey,
                    SitesByKey = sitesByKey,
                    GeomsByKey = geomsByKey,
                    Names = names,
                    MeshIds = meshIds,
                    UsedBodyNames = new HashSet<string>(),
                    BodyNames = new Dictionary<Transform, string>(),
                };
                int bodyCount = EmitBody(model.transform, "", 0, root.rootMobility == RootMobilityMode.Free, context);
                writer.WriteEndElement();

                if (actuators.Count > 0)
                {
                    writer.WriteStartElement("actuator");
                    foreach (SO.Actuator actuator in actuators)
                    {
                        WriteActuator(writer, w, actuator, names);
                    }
                    writer.WriteEndElement();
                }

                if (sensors.Count > 0)
                {
                    writer.WriteStartElement("sensor");
                    foreach (SO.Sensor sensor in sensors)
                    {
                        WriteSensor(writer, w, sensor, names.Sensors[sensor], names, model, context.BodyNames);
                    }
                    writer.WriteEndElement();
                }

                writer.WriteEndElement();
                writer.WriteEndDocument();
                return bodyCount;
            }
        }

        private static void WriteOption(XmlWriter writer, Attributes w, SO.Root root)
        {
            writer.WriteStartElement("option");
            writer.WriteAttributeString("timestep", Format(root.timestepSeconds));
            w.Float("impratio", root.impratio, 1f);
            w.Float("tolerance", root.tolerance, 1e-8f);
            w.Float("ls_tolerance", root.ls_tolerance, 0.01f);
            w.Float("noslip_tolerance", root.noslip_tolerance, 1e-6f);
            w.Float("ccd_tolerance", root.ccd_tolerance, 1e-6f);
            w.Float("sleep_tolerance", root.sleep_tolerance, 0.001f);
            w.Point("gravity", root.gravity, new Vector3(0f, -9.81f, 0f));
            w.Point("wind", root.wind, Vector3.zero);
            w.Point("magnetic", root.magnetic, new Vector3(0f, 0f, -0.5f));
            w.Float("density", root.density, 0f);
            w.Float("viscosity", root.viscosity, 0f);
            w.Float("o_margin", root.o_margin, 0f);
            w.Floats("o_solref", root.o_solref, new[] { 0.02f, 1f });
            w.Floats("o_solimp", root.o_solimp, new[] { 0.9f, 0.95f, 0.001f, 0.5f, 2f });
            w.Floats("o_friction", root.o_friction, new[] { 1f, 1f, 0.005f, 0.0001f, 0.0001f });
            writer.WriteAttributeString("integrator", IntegratorKeyword(root.integrator));
            w.Keyword("cone", root.cone, Cone.Pyramidal);
            w.Keyword("jacobian", root.jacobian, Jacobian.Auto);
            // Solver keywords are case-sensitive and match the enum names.
            if (root.solver != Solver.Newton)
            {
                writer.WriteAttributeString("solver", root.solver.ToString());
            }
            w.Int("iterations", root.iterations, 100);
            w.Int("ls_iterations", root.ls_iterations, 50);
            w.Int("noslip_iterations", root.noslip_iterations, 0);
            w.Int("ccd_iterations", root.ccd_iterations, 35);
            w.Int("sdf_iterations", root.sdf_iterations, 10);
            w.Int("sdf_initpoints", root.sdf_initpoints, 40);
            if (root.actuatorgroupdisable != null && root.actuatorgroupdisable.Length > 0)
            {
                writer.WriteAttributeString(
                    "actuatorgroupdisable",
                    string.Join(" ", root.actuatorgroupdisable.Select(value => value.ToString(CultureInfo.InvariantCulture))));
            }
            WriteFlag(writer, root.flag ?? new Flag());
            writer.WriteEndElement();
        }

        // <flag> is written only when a switch differs from its default,
        // and then with only the differing switches.
        private static void WriteFlag(XmlWriter writer, Flag flag)
        {
            var defaults = new Flag();
            var changed = new List<(string name, EnableState value)>();
            void Check(string name, EnableState value, EnableState fallback)
            {
                if (value != fallback)
                {
                    changed.Add((name, value));
                }
            }
            Check("constraint", flag.constraint, defaults.constraint);
            Check("equality", flag.equality, defaults.equality);
            Check("frictionloss", flag.frictionloss, defaults.frictionloss);
            Check("limit", flag.limit, defaults.limit);
            Check("contact", flag.contact, defaults.contact);
            Check("spring", flag.spring, defaults.spring);
            Check("damper", flag.damper, defaults.damper);
            Check("gravity", flag.gravity, defaults.gravity);
            Check("clampctrl", flag.clampctrl, defaults.clampctrl);
            Check("warmstart", flag.warmstart, defaults.warmstart);
            Check("filterparent", flag.filterparent, defaults.filterparent);
            Check("actuation", flag.actuation, defaults.actuation);
            Check("refsafe", flag.refsafe, defaults.refsafe);
            Check("sensor", flag.sensor, defaults.sensor);
            Check("midphase", flag.midphase, defaults.midphase);
            Check("eulerdamp", flag.eulerdamp, defaults.eulerdamp);
            Check("autoreset", flag.autoreset, defaults.autoreset);
            Check("nativeccd", flag.nativeccd, defaults.nativeccd);
            Check("island", flag.island, defaults.island);
            Check("multiccd", flag.multiccd, defaults.multiccd);
            Check("override", flag.@override, defaults.@override);
            Check("energy", flag.energy, defaults.energy);
            Check("fwdinv", flag.fwdinv, defaults.fwdinv);
            Check("invdiscrete", flag.invdiscrete, defaults.invdiscrete);
            Check("sleep", flag.sleep, defaults.sleep);
            Check("diagexact", flag.diagexact, defaults.diagexact);
            Check("ipc", flag.ipc, defaults.ipc);
            if (changed.Count == 0)
            {
                return;
            }
            writer.WriteStartElement("flag");
            foreach ((string name, EnableState value) in changed)
            {
                writer.WriteAttributeString(name, value == EnableState.Enable ? "enable" : "disable");
            }
            writer.WriteEndElement();
        }

        private sealed class Context
        {
            public XmlWriter Writer;
            public Attributes W;
            public Dictionary<string, List<SO.Joint>> JointsByKey;
            public Dictionary<string, List<SO.Site>> SitesByKey;
            public Dictionary<string, List<SO.Geom>> GeomsByKey;
            public Names Names;
            public Dictionary<Mesh, int> MeshIds;
            public HashSet<string> UsedBodyNames;
            // Each GameObject's exported <body> name, for sensors that name a
            // body (written after the worldbody).
            public Dictionary<Transform, string> BodyNames;
        }

        private static int EmitBody(Transform node, string pathKey, int nodeIndex, bool freeRoot, Context context)
        {
            XmlWriter writer = context.Writer;
            Attributes w = context.W;
            writer.WriteComment($" part: {CommentSafe(node.name)} ");
            writer.WriteStartElement("body");
            string bodyName = Dedupe(Sanitize(node.name), context.UsedBodyNames, () => $"part{nodeIndex}");
            context.BodyNames[node] = bodyName;
            writer.WriteAttributeString("name", bodyName);
            writer.WriteAttributeString("pos", FormatPoint(node.localPosition));
            writer.WriteAttributeString("quat", FormatQuat(node.localRotation));

            if (freeRoot)
            {
                writer.WriteStartElement("freejoint");
                writer.WriteEndElement();
            }

            if (context.JointsByKey.TryGetValue(pathKey, out List<SO.Joint> joints))
            {
                foreach (SO.Joint joint in joints)
                {
                    WriteJoint(writer, w, context.Names.Joints[joint], joint.joint);
                }
            }

            if (context.SitesByKey.TryGetValue(pathKey, out List<SO.Site> sites))
            {
                foreach (SO.Site site in sites)
                {
                    WriteSite(writer, w, context.Names.Sites[site], site.site);
                }
            }

            MeshFilter filter = node.GetComponent<MeshFilter>();
            string partMesh = filter != null && filter.sharedMesh != null
                && context.MeshIds.TryGetValue(filter.sharedMesh, out int meshId)
                ? MeshName(meshId)
                : null;
            foreach ((Geom geom, SO.Geom asset) in GeomsOf(node, pathKey, context.GeomsByKey))
            {
                WriteGeom(writer, w, asset != null ? context.Names.Geoms[asset] : null, geom, asset == null ? partMesh : null);
            }

            Inertial inertial = DefaultInertial(node, pathKey, context.GeomsByKey);
            if (inertial != null)
            {
                WriteInertial(writer, w, inertial);
            }

            int bodyCount = 1;
            for (int i = 0; i < node.childCount; i++)
            {
                bodyCount += EmitBody(node.GetChild(i), ChildKey(pathKey, i), nodeIndex + bodyCount, false, context);
            }

            writer.WriteEndElement();
            return bodyCount;
        }

        private static void WriteJoint(XmlWriter writer, Attributes w, string name, Joint joint)
        {
            writer.WriteStartElement("joint");
            writer.WriteAttributeString("name", name);
            w.Text("class", joint.@class);
            w.Keyword("type", joint.type, JointType.Hinge);
            w.Int("group", joint.group, 0);
            w.Point("pos", joint.pos, Vector3.zero);
            w.Point("axis", joint.axis, new Vector3(0f, 1f, 0f));
            if (joint.springdamper != Vector2.zero)
            {
                writer.WriteAttributeString("springdamper", $"{Format(joint.springdamper.x)} {Format(joint.springdamper.y)}");
            }
            w.Keyword("limited", joint.limited, FalseTrueAuto.Auto);
            w.Keyword("actuatorfrclimited", joint.actuatorfrclimited, FalseTrueAuto.Auto);
            w.Floats("solreflimit", joint.solreflimit, new[] { 0.02f, 1f });
            w.Floats("solimplimit", joint.solimplimit, new[] { 0.9f, 0.95f, 0.001f, 0.5f, 2f });
            w.Floats("solreffriction", joint.solreffriction, new[] { 0.02f, 1f });
            w.Floats("solimpfriction", joint.solimpfriction, new[] { 0.9f, 0.95f, 0.001f, 0.5f, 2f });
            w.Floats("stiffness", joint.stiffness, new[] { 0f });
            w.Range("range", joint.rangeLo, joint.rangeHi, joint.limited == FalseTrueAuto.True);
            w.Range("actuatorfrcrange", joint.actuatorfrcrangeLo, joint.actuatorfrcrangeHi, joint.actuatorfrclimited == FalseTrueAuto.True);
            w.Bool("actuatorgravcomp", joint.actuatorgravcomp, false);
            w.Float("margin", joint.margin, 0f);
            w.Float("ref", joint.@ref, 0f);
            w.Float("springref", joint.springref, 0f);
            w.Float("armature", joint.armature, 0f);
            w.Floats("damping", joint.damping, new[] { 0f });
            w.Float("frictionloss", joint.frictionloss, 0f);
            w.Floats("user", joint.user, Array.Empty<float>());
            writer.WriteEndElement();
        }

        private static void WriteSite(XmlWriter writer, Attributes w, string name, Site site)
        {
            writer.WriteStartElement("site");
            writer.WriteAttributeString("name", name);
            w.Text("class", site.@class);
            w.Keyword("type", site.type, SiteType.Sphere);
            w.Int("group", site.group, 0);
            if (!site.useFromto)
            {
                w.Point("pos", site.pos, Vector3.zero);
                w.Quat("quat", site.quat);
            }
            w.Text("material", site.material);
            w.Text("mesh", site.mesh);
            w.Size("size", site.size, GeomTypeOf(site.type), new Vector3(0.005f, 0.005f, 0.005f));
            if (site.useFromto)
            {
                writer.WriteAttributeString("fromto", $"{FormatPoint(site.fromtoFrom)} {FormatPoint(site.fromtoTo)}");
            }
            w.Rgba("rgba", site.rgba, new Color(0.5f, 0.5f, 0.5f, 1f));
            w.Floats("user", site.user, Array.Empty<float>());
            writer.WriteEndElement();
        }

        // partMesh is the GameObject's exported MeshFilter mesh, passed for
        // its default geom only; a Geom asset never falls back to it. Used
        // when a Mesh geom
        // names no mesh of its own. Default geoms are unnamed.
        private static void WriteGeom(XmlWriter writer, Attributes w, string name, Geom geom, string partMesh)
        {
            writer.WriteStartElement("geom");
            w.Text("name", name);
            w.Text("class", geom.@class);
            w.Keyword("type", geom.type, GeomType.Sphere);
            w.Int("contype", geom.contype, 1);
            w.Int("conaffinity", geom.conaffinity, 1);
            w.Int("condim", geom.condim, 3);
            w.Int("group", geom.group, 0);
            w.Int("priority", geom.priority, 0);
            w.Size("size", geom.size, geom.type, Vector3.zero);
            w.Text("material", geom.material);
            w.Floats("friction", geom.friction, new[] { 1f, 0.005f, 0.0001f });
            w.Float("mass", geom.mass, 0f);
            w.Float("density", geom.density, 1000f);
            w.Bool("shellinertia", geom.shellinertia, false);
            w.Float("solmix", geom.solmix, 1f);
            w.Floats("solref", geom.solref, new[] { 0.02f, 1f });
            w.Floats("solimp", geom.solimp, new[] { 0.9f, 0.95f, 0.001f, 0.5f, 2f });
            w.Float("margin", geom.margin, 0f);
            w.Float("gap", geom.gap, 0f);
            w.Spatial("surfacevel", geom.surfacevel);
            w.Float("adhesion", geom.adhesion, 0f);
            if (geom.useFromto)
            {
                writer.WriteAttributeString("fromto", $"{FormatPoint(geom.fromtoFrom)} {FormatPoint(geom.fromtoTo)}");
            }
            else
            {
                w.Point("pos", geom.pos, Vector3.zero);
                w.Quat("quat", geom.quat);
            }
            w.Text("hfield", geom.hfield);
            string mesh = !string.IsNullOrEmpty(geom.mesh) ? geom.mesh : geom.type == GeomType.Mesh ? partMesh : null;
            w.Text("mesh", mesh);
            w.Float("fitscale", geom.fitscale, 1f);
            w.Rgba("rgba", geom.rgba, new Color(0.5f, 0.5f, 0.5f, 1f));
            w.Keyword("fluidshape", geom.fluidshape, FluidShape.None);
            w.Floats("fluidcoef", geom.fluidcoef, new[] { 0.5f, 0.25f, 1.5f, 1f, 1f });
            w.Floats("user", geom.user, Array.Empty<float>());
            writer.WriteEndElement();
        }

        // pos and mass are required by the schema, so always written.
        private static void WriteInertial(XmlWriter writer, Attributes w, Inertial inertial)
        {
            writer.WriteStartElement("inertial");
            writer.WriteAttributeString("pos", FormatPoint(inertial.pos));
            writer.WriteAttributeString("mass", Format(inertial.mass));
            if (inertial.useFullinertia && inertial.fullinertia is { Length: 6 })
            {
                // Unity [Ixx, Iyy, Izz, Ixy, Ixz, Iyz] -> MJCF
                // [Ixx, Izz, Iyy, Ixz, Ixy, Iyz] (see Inertial).
                float[] i = inertial.fullinertia;
                writer.WriteAttributeString(
                    "fullinertia",
                    $"{Format(i[0])} {Format(i[2])} {Format(i[1])} {Format(i[4])} {Format(i[3])} {Format(i[5])}");
            }
            else
            {
                w.Quat("quat", inertial.quat);
                Vector3 d = inertial.diaginertia;
                writer.WriteAttributeString("diaginertia", $"{Format(d.x)} {Format(d.y)} {Format(d.z)}");
            }
            writer.WriteEndElement();
        }

        private static void WriteActuator(XmlWriter writer, Attributes w, SO.Actuator asset, Names names)
        {
            Actuator actuator = asset.actuator;
            writer.WriteStartElement(ActuatorElement(actuator.type));
            writer.WriteAttributeString("name", names.Actuators[asset]);
            w.Text("class", actuator.@class);
            w.Int("group", actuator.group, 0);
            w.Int("nsample", actuator.nsample, 0);
            w.Keyword("interp", actuator.interp, Interp.Zoh);
            w.Float("delay", actuator.delay, 0f);
            w.Range("ctrlrange", actuator.ctrlLo, actuator.ctrlHi, actuator.ctrllimited == FalseTrueAuto.True);
            w.Floats("user", actuator.user, Array.Empty<float>());
            w.Keyword("ctrllimited", actuator.ctrllimited, FalseTrueAuto.Auto);
            w.Keyword("forcelimited", actuator.forcelimited, FalseTrueAuto.Auto);
            if (actuator.type == ActuatorType.Position)
            {
                w.Float("inheritrange", actuator.inheritrange, 0f);
            }
            w.Range("forcerange", actuator.forceLo, actuator.forceHi, actuator.forcelimited == FalseTrueAuto.True);
            w.Range("lengthrange", actuator.lengthrangeLo, actuator.lengthrangeHi, false);
            w.Floats("gear", actuator.gear, new[] { 1f, 0f, 0f, 0f, 0f, 0f });
            w.Floats("damping", actuator.damping, new[] { 0f });
            w.Float("armature", actuator.armature, 0f);
            // cranklength's presence is an error outside a slider-crank,
            // even at 0, so it is written only when nonzero.
            w.Float("cranklength", actuator.cranklength, 0f);

            if (asset.target != null)
            {
                writer.WriteAttributeString("joint", names.Joints[asset.target]);
            }
            if (asset.jointinparent != null)
            {
                writer.WriteAttributeString("jointinparent", names.Joints[asset.jointinparent]);
            }
            if (asset.slidersite != null)
            {
                writer.WriteAttributeString("slidersite", names.Sites[asset.slidersite]);
            }
            if (asset.cranksite != null)
            {
                writer.WriteAttributeString("cranksite", names.Sites[asset.cranksite]);
            }
            if (asset.site != null)
            {
                writer.WriteAttributeString("site", names.Sites[asset.site]);
            }
            if (asset.refsite != null)
            {
                writer.WriteAttributeString("refsite", names.Sites[asset.refsite]);
            }

            if (actuator.type == ActuatorType.Position)
            {
                writer.WriteAttributeString("kp", Format(actuator.kp));
                // kv and dampratio are presence-exclusive: the compiler
                // rejects both, even at defaults. dampratio wins.
                if (actuator.dampratio != 0f)
                {
                    writer.WriteAttributeString("dampratio", Format(actuator.dampratio));
                }
                else
                {
                    w.Float("kv", actuator.kv, 0f);
                }
                w.Float("timeconst", actuator.timeconst, 0f);
            }
            else
            {
                writer.WriteAttributeString("kv", Format(actuator.kv));
            }
            writer.WriteEndElement();
        }

        // framepos: the objtype/objname object's position in world
        // coordinates, or in the reftype/refname object's frame when one is
        // set (both written together, or neither).
        private static void WriteSensor(
            XmlWriter writer,
            Attributes w,
            SO.Sensor asset,
            string name,
            Names names,
            GameObject model,
            Dictionary<Transform, string> bodyNames)
        {
            Sensor sensor = asset.sensor ?? new Sensor();
            writer.WriteStartElement("framepos");
            writer.WriteAttributeString("name", name);
            w.Int("nsample", sensor.nsample, 0);
            w.Keyword("interp", sensor.interp, Interp.Zoh);
            w.Float("delay", sensor.delay, 0f);
            w.Floats("interval", sensor.interval, Array.Empty<float>());
            w.Float("cutoff", sensor.cutoff, 0f);
            w.Float("noise", sensor.noise, 0f);
            w.Floats("user", sensor.user, Array.Empty<float>());
            string objname = FrameName(asset.objtype, asset.objnameSite, asset.objnameGeom, asset.objnameBody, names, model, bodyNames);
            writer.WriteAttributeString("objtype", asset.objtype.ToString().ToLowerInvariant());
            writer.WriteAttributeString("objname", objname);
            string refname = FrameName(asset.reftype, asset.refnameSite, asset.refnameGeom, asset.refnameBody, names, model, bodyNames);
            if (refname != null)
            {
                writer.WriteAttributeString("reftype", asset.reftype.ToString().ToLowerInvariant());
                writer.WriteAttributeString("refname", refname);
            }
            writer.WriteEndElement();
        }

        // The MJCF name of a <framepos> objname or refname, by its type:
        // the Site's or Geom's exported name, or the exported <body> name
        // of the GameObject for Body and XBody. Null when the field for
        // that type is empty (for a reference: world, nothing written).
        private static string FrameName(
            FrameObject type,
            SO.Site site,
            SO.Geom geom,
            SiblingPath body,
            Names names,
            GameObject model,
            Dictionary<Transform, string> bodyNames)
        {
            switch (type)
            {
                case FrameObject.Site:
                    return site != null ? names.Sites[site] : null;
                case FrameObject.Geom:
                    return geom != null ? names.Geoms[geom] : null;
                case FrameObject.Body:
                case FrameObject.XBody:
                    if (body == null || body.root == null || !HasTarget(body))
                    {
                        return null;
                    }
                    Transform node = Resolve(model.transform, body);
                    return node != null && bodyNames.TryGetValue(node, out string bodyName) ? bodyName : null;
                default:
                    return null;
            }
        }

        // Binary STL from the Unity mesh: vertices are body-local meters
        // in Unity axes, so the export is the (x, z, y) swap. The importer
        // flipped two indices per triangle for Unity's winding; flipping
        // again recovers the right-handed order STL stores, and the facet
        // normal follows that winding. MuJoCo recenters meshes itself.
        private static void WriteStl(string path, Mesh mesh)
        {
            Vector3[] vertices = mesh.vertices;
            int[] triangles = mesh.triangles;
            using (FileStream stream = File.Create(path))
            using (var writer = new BinaryWriter(stream))
            {
                writer.Write(new byte[80]);
                writer.Write(triangles.Length / 3);
                for (int t = 0; t < triangles.Length; t += 3)
                {
                    Vector3 a = Swap(vertices[triangles[t]]);
                    Vector3 b = Swap(vertices[triangles[t + 2]]);
                    Vector3 c = Swap(vertices[triangles[t + 1]]);
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

        private static void WriteFloat3(BinaryWriter writer, Vector3 value)
        {
            writer.Write(value.x);
            writer.Write(value.y);
            writer.Write(value.z);
        }

        // ------------------------------------------------------------ names

        // MJCF names: the payload's `name`, or the asset's name when that is
        // empty; sanitized and made unique per element kind.
        private sealed class Names
        {
            public readonly Dictionary<SO.Joint, string> Joints = new Dictionary<SO.Joint, string>();
            public readonly Dictionary<SO.Site, string> Sites = new Dictionary<SO.Site, string>();
            public readonly Dictionary<SO.Geom, string> Geoms = new Dictionary<SO.Geom, string>();
            public readonly Dictionary<SO.Actuator, string> Actuators = new Dictionary<SO.Actuator, string>();
            public readonly Dictionary<SO.Sensor, string> Sensors = new Dictionary<SO.Sensor, string>();

            public Names(SO.Root root)
            {
                Fill(root.joints, Joints, element => element.joint?.name, "joint");
                Fill(root.sites, Sites, element => element.site?.name, "site");
                Fill(root.geoms, Geoms, element => element.geom?.name, "geom");
                Fill(root.actuators, Actuators, element => element.actuator?.name, "actuator");
                Fill(root.sensors, Sensors, element => element.sensor?.name, "sensor");
            }

            private static void Fill<T>(
                List<T> elements, Dictionary<T, string> names, Func<T, string> payloadName, string fallback)
                where T : UnityEngine.Object
            {
                var used = new HashSet<string>();
                if (elements == null)
                {
                    return;
                }
                foreach (T element in elements)
                {
                    if (element != null && !names.ContainsKey(element))
                    {
                        string authored = payloadName(element);
                        string name = string.IsNullOrEmpty(authored) ? element.name : authored;
                        names.Add(element, Dedupe(Sanitize(name), used, () => fallback));
                    }
                }
            }
        }

        // -------------------------------------------------------- attributes

        // Writes an attribute only when its value differs from the given
        // schema default.
        private sealed class Attributes
        {
            private readonly XmlWriter m_Writer;

            public Attributes(XmlWriter writer)
            {
                m_Writer = writer;
            }

            public void Text(string name, string value)
            {
                if (!string.IsNullOrEmpty(value))
                {
                    m_Writer.WriteAttributeString(name, value);
                }
            }

            public void Int(string name, int value, int fallback)
            {
                if (value != fallback)
                {
                    m_Writer.WriteAttributeString(name, value.ToString(CultureInfo.InvariantCulture));
                }
            }

            public void Float(string name, float value, float fallback)
            {
                if (value != fallback)
                {
                    m_Writer.WriteAttributeString(name, Format(value));
                }
            }

            public void Bool(string name, bool value, bool fallback)
            {
                if (value != fallback)
                {
                    m_Writer.WriteAttributeString(name, value ? "true" : "false");
                }
            }

            // Enum keyword = the member name in lower case (e.g. Hfield ->
            // "hfield", ImplicitFast is not used here).
            public void Keyword<TEnum>(string name, TEnum value, TEnum fallback) where TEnum : struct, Enum
            {
                if (!EqualityComparer<TEnum>.Default.Equals(value, fallback))
                {
                    m_Writer.WriteAttributeString(name, value.ToString().ToLowerInvariant());
                }
            }

            public void Floats(string name, float[] values, float[] fallback)
            {
                if (values != null && values.Length > 0 && !values.SequenceEqual(fallback))
                {
                    m_Writer.WriteAttributeString(name, string.Join(" ", values.Select(Format)));
                }
            }

            // A point or direction in Unity axes, written as (x, z, y).
            public void Point(string name, Vector3 value, Vector3 fallback)
            {
                if (value != fallback)
                {
                    m_Writer.WriteAttributeString(name, FormatPoint(value));
                }
            }

            public void Quat(string name, Quaternion value)
            {
                if (value.x != 0f || value.y != 0f || value.z != 0f || value.w != 1f)
                {
                    m_Writer.WriteAttributeString(name, FormatQuat(value));
                }
            }

            public void Rgba(string name, Color value, Color fallback)
            {
                if (value != fallback)
                {
                    m_Writer.WriteAttributeString(name, $"{Format(value.r)} {Format(value.g)} {Format(value.b)} {Format(value.a)}");
                }
            }

            // A two-value range: written when either bound is nonzero, or
            // when its limit switch is explicitly true.
            public void Range(string name, float lo, float hi, bool forced)
            {
                if (forced || lo != 0f || hi != 0f)
                {
                    m_Writer.WriteAttributeString(name, $"{Format(lo)} {Format(hi)}");
                }
            }

            // Shape sizes. Box half-sizes and ellipsoid radii are per axis,
            // so they are swapped like a point. For the other types the
            // values are a parameter list (sphere: radius; capsule and
            // cylinder: radius, half-length along the geom's own z; plane:
            // half-sizes and grid spacing) and are written as authored.
            public void Size(string name, Vector3 value, GeomType type, Vector3 fallback)
            {
                if (value == fallback)
                {
                    return;
                }
                bool perAxis = type == GeomType.Box || type == GeomType.Ellipsoid;
                Vector3 written = perAxis ? Swap(value) : value;
                m_Writer.WriteAttributeString(name, $"{Format(written.x)} {Format(written.y)} {Format(written.z)}");
            }

            // Spatial lists: 3 values swap as a point; 6 values as two
            // points (linear then angular); other lengths as authored.
            // Unset (not written) when empty or all zeros: the schema gives
            // surfacevel no default, and zero velocity is MuJoCo's behaviour
            // without it. The step importer's default geom stores six zeros.
            public void Spatial(string name, float[] values)
            {
                if (values == null || values.Length == 0 || values.All(value => value == 0f))
                {
                    return;
                }
                float[] written = (float[])values.Clone();
                if (written.Length == 3 || written.Length == 6)
                {
                    for (int start = 0; start < written.Length; start += 3)
                    {
                        (written[start + 1], written[start + 2]) = (written[start + 2], written[start + 1]);
                    }
                }
                m_Writer.WriteAttributeString(name, string.Join(" ", written.Select(Format)));
            }
        }

        // ------------------------------------------------------------ utils

        // A path with a target: indices [SELF] (the root itself) or child
        // indices. Empty indices mean no target was set.
        private static bool HasTarget(SiblingPath path)
        {
            return path.indices != null && path.indices.Length > 0;
        }

        private static bool IsSelf(int[] indices)
        {
            return indices != null && indices.Length == 1 && indices[0] == SiblingPath.SELF;
        }

        // The GameObject a path names: root for [SELF], else the walk down
        // the child indices. Null for no target or a path that no longer
        // fits the hierarchy.
        private static Transform Resolve(Transform root, SiblingPath path)
        {
            if (path == null || !HasTarget(path))
            {
                return null;
            }
            if (IsSelf(path.indices))
            {
                return root;
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
            return node;
        }

        // The per-GameObject key elements are grouped by: "2/0" for child
        // indices, "" for the root (indices [SELF]), the same key EmitBody
        // gives the root body.
        private static string KeyOf(int[] indices)
        {
            return indices == null || IsSelf(indices) ? "" : string.Join("/", indices);
        }

        private static string ChildKey(string pathKey, int index)
        {
            return pathKey.Length == 0 ? index.ToString(CultureInfo.InvariantCulture) : $"{pathKey}/{index}";
        }

        private static string MeshName(int index)
        {
            return $"mesh_{index:D3}";
        }

        // SiteType and GeomType name the same shapes; Size needs GeomType.
        private static GeomType GeomTypeOf(SiteType type)
        {
            return (GeomType)Enum.Parse(typeof(GeomType), type.ToString());
        }

        // The option/integrator keyword as MuJoCo's parser spells it
        // (case-sensitive: "Euler", "RK4", "implicit", "implicitfast",
        // "discrete"); null for a value the enum does not define.
        private static string IntegratorKeyword(Integrator integrator)
        {
            switch (integrator)
            {
                case Integrator.Euler:
                    return "Euler";
                case Integrator.RK4:
                    return "RK4";
                case Integrator.Implicit:
                    return "implicit";
                case Integrator.ImplicitFast:
                    return "implicitfast";
                case Integrator.Discrete:
                    return "discrete";
                default:
                    return null;
            }
        }

        private static string ActuatorElement(ActuatorType type)
        {
            switch (type)
            {
                case ActuatorType.Position:
                    return "position";
                case ActuatorType.Velocity:
                    return "velocity";
                default:
                    return null;
            }
        }

        private static Vector3 Swap(Vector3 unity)
        {
            return new Vector3(unity.x, unity.z, unity.y);
        }

        private static string FormatPoint(Vector3 unity)
        {
            Vector3 value = Swap(unity);
            return $"{Format(value.x)} {Format(value.y)} {Format(value.z)}";
        }

        // A Unity rotation written as an MJCF quaternion (w x y z).
        // Swapping y and z is a reflection, so it turns a rotation by θ
        // about axis a into a rotation by -θ about the swapped axis:
        // (x, y, z, w) becomes (-x, -z, -y, w). Worked example: 90° about
        // Unity y, (0, 0.7071, 0, 0.7071), becomes -90° about MJCF z,
        // written "0.7071 0 0 -0.7071".
        private static string FormatQuat(Quaternion unity)
        {
            return $"{Format(unity.w)} {Format(-unity.x)} {Format(-unity.z)} {Format(-unity.y)}";
        }

        // MJCF names are free strings; the character restriction keeps
        // names predictable and comment-safe.
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

        private static string Dedupe(string name, HashSet<string> used, Func<string> fallback)
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

        private static string Format(float value)
        {
            return value.ToString("R", CultureInfo.InvariantCulture);
        }
    }
}
