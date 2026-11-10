using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Actuator inspector. Layout in UI/Actuator.uxml, styles in
    // UI/Actuator.uss, both assigned as this script's default references
    // (its .meta), as for JointEditor. Top to bottom: the identity card
    // (name, type) with the Actuators bar colour, Transmission, Gains and
    // Control (open), then Force, Dynamics, Delay and Other (collapsed).
    // Labels are the MJCF attribute names, with their SI unit in
    // parentheses where it is known (Units below).
    //
    // Fields MuJoCo rejects on <velocity> (kp, dampratio, timeconst,
    // inheritrange: "Schema violation: unrecognized attribute") carry the
    // UXML class "position-only" and are hidden while type is Velocity.
    // Their values stay stored; the exporter does not write them for a
    // Velocity actuator.
    //
    // Naming: whenever `target` changes to a joint whose body path has a
    // root, an empty MJCF `name` becomes "a-[<indices>]-<targetName>" from
    // that joint's body path, indices comma-separated. Example: a target
    // joint on part "Pole" at [2,0] gives "a-[2,0]-Pole". A name already
    // set is never replaced. Setting the name renames the asset to match
    // (ElementEditor). The exporter turns [ , ] into _, so "a-[2,0]-Pole"
    // exports as "a-_2_0_-Pole".
    [CustomEditor(typeof(SO.Actuator))]
    internal sealed class ActuatorEditor : ElementEditor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        protected override string NamePath => "actuator.name";

        protected override VisualElement BuildLayout()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(ActuatorEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(ActuatorEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            SerializedProperty type = serializedObject.FindProperty("actuator.type");
            void ShowForType(SerializedProperty property)
            {
                bool position = property.enumValueIndex == (int)ActuatorType.Position;
                root.Query<VisualElement>(className: "position-only")
                    .ForEach(field => field.style.display = position ? DisplayStyle.Flex : DisplayStyle.None);
            }
            ShowForType(type);
            root.TrackPropertyValue(type, ShowForType);

            // Units follow the type, the target joint and the gear, so the
            // labels are rewritten whenever any of them changes.
            void RefreshUnits(SerializedProperty _)
            {
                WriteUnits(root);
            }
            RefreshUnits(null);
            root.TrackPropertyValue(type, RefreshUnits);
            root.TrackPropertyValue(serializedObject.FindProperty("target"), RefreshUnits);
            root.TrackPropertyValue(serializedObject.FindProperty("actuator.gear"), RefreshUnits);
            return root;
        }

        // SI units of the quantities an actuator on one joint touches (the
        // repo's convention: meters, kilograms, seconds, radians; MuJoCo's
        // defaults are SI and our exports write angle="radian"). On a joint
        // transmission the actuator length is gear[0]·qpos, so with
        // gear[0] = 1 control, ranges and kp are in the joint's own units:
        // a slide works in m, m/s, N; a hinge in rad, rad/s, N·m. With no
        // target joint, a ball or free joint, or gear[0] ≠ 1 (the length is
        // scaled), the joint-dependent units are left off rather than
        // shown wrong; delay and timeconst (s), cranklength (m),
        // dampratio and inheritrange are always shown.
        private sealed class Units
        {
            public string Position = "";
            public string Velocity = "";
            public string Force = "";
            public string Stiffness = "";
            public string Damping = "";
            public string VelocityGain = "";
            public string Inertia = "";
        }

        private Units UnitsOfTarget()
        {
            var actuator = (SO.Actuator)target;
            SO.Joint joint = actuator != null ? actuator.target : null;
            float[] gear = actuator != null ? actuator.actuator.gear : null;
            if (joint == null || (gear != null && gear.Length > 0 && gear[0] != 1f))
            {
                return new Units();
            }
            switch (joint.joint.type)
            {
                case JointType.Slide:
                    return new Units
                    {
                        Position = "m", Velocity = "m/s", Force = "N", Stiffness = "N/m",
                        Damping = "N·s/m", VelocityGain = "N/(m/s)", Inertia = "kg",
                    };
                case JointType.Hinge:
                    return new Units
                    {
                        Position = "rad", Velocity = "rad/s", Force = "N·m", Stiffness = "N·m/rad",
                        Damping = "N·m·s/rad", VelocityGain = "N·m/(rad/s)", Inertia = "kg·m²",
                    };
                default:
                    return new Units();
            }
        }

        private void WriteUnits(VisualElement root)
        {
            Units units = UnitsOfTarget();
            bool velocity = ((SO.Actuator)target).actuator.type == ActuatorType.Velocity;
            string control = velocity ? units.Velocity : units.Position;
            SetLabel(root, "unit-ctrlLo", "ctrlrange lo", control);
            SetLabel(root, "unit-ctrlHi", "ctrlrange hi", control);
            SetLabel(root, "unit-forceLo", "forcerange lo", units.Force);
            SetLabel(root, "unit-forceHi", "forcerange hi", units.Force);
            SetLabel(root, "unit-lengthrangeLo", "lengthrange lo", units.Position);
            SetLabel(root, "unit-lengthrangeHi", "lengthrange hi", units.Position);
            SetLabel(root, "unit-kp", "kp", units.Stiffness);
            SetLabel(root, "unit-kv", "kv", velocity ? units.VelocityGain : units.Damping);
            SetLabel(root, "unit-damping", "damping", units.Damping);
            SetLabel(root, "unit-armature", "armature", units.Inertia);
            SetLabel(root, "unit-delay", "delay", "s");
            SetLabel(root, "unit-timeconst", "timeconst", "s");
            SetLabel(root, "unit-cranklength", "cranklength", "m");
            SetLabel(root, "unit-dampratio", "dampratio", "1 = critical");
            SetLabel(root, "unit-inheritrange", "inheritrange", "× joint range");
        }


        protected override void Extend(VisualElement container)
        {
            container.TrackPropertyValue(
                serializedObject.FindProperty("target"), _ => FillName("a-", ((SO.Actuator)target).target?.body));
        }
    }
}
