using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The actuator asset's inspector: a bound UXML form. The UI assets
    // are serialized references assigned on this script asset —
    // package-relative Style src is unreliable, and the GUID in the
    // shipped .meta keeps the assignments stable. The form is the MJCF
    // mirror and draws only the selected type's attributes: Position
    // shows kp, kv (as damping), dampratio, timeconst, inheritrange;
    // Velocity shows kv (as its gain) and none of the position-only
    // four. Every numeric label carries its unit, which depends on the
    // actuator type and on the target joint's type (Units below).
    [CustomEditor(typeof(StepActuator))]
    internal sealed class StepActuatorEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepActuatorEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepActuatorEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            var form = new Form
            {
                Delay = root.Q<FloatField>("delay"),
                CtrlLo = root.Q<FloatField>("ctrl-lo"),
                CtrlHi = root.Q<FloatField>("ctrl-hi"),
                ForceLo = root.Q<FloatField>("force-lo"),
                ForceHi = root.Q<FloatField>("force-hi"),
                LengthrangeLo = root.Q<FloatField>("lengthrange-lo"),
                LengthrangeHi = root.Q<FloatField>("lengthrange-hi"),
                Cranklength = root.Q<FloatField>("cranklength"),
                Kp = root.Q<FloatField>("kp"),
                Kv = root.Q<FloatField>("kv"),
                Dampratio = root.Q<FloatField>("dampratio"),
                Timeconst = root.Q<FloatField>("timeconst"),
                Inheritrange = root.Q<FloatField>("inheritrange"),
                Damping = root.Q<FloatField>("damping"),
                Armature = root.Q<FloatField>("armature"),
                ExclusivityWarning = root.Q<Label>("exclusivity-warning"),
                VelocityGainWarning = root.Q<Label>("velocity-gain-warning"),
            };
            UpdateForm(form);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change => UpdateForm(form));
            return root;
        }

        // The form's type- and value-dependent elements. The numeric
        // fields are FloatFields (not PropertyFields) because their
        // label is a plain property that can be rewritten with the units.
        private sealed class Form
        {
            public FloatField Delay;
            public FloatField CtrlLo;
            public FloatField CtrlHi;
            public FloatField ForceLo;
            public FloatField ForceHi;
            public FloatField LengthrangeLo;
            public FloatField LengthrangeHi;
            public FloatField Cranklength;
            public FloatField Kp;
            public FloatField Kv;
            public FloatField Dampratio;
            public FloatField Timeconst;
            public FloatField Inheritrange;
            public FloatField Damping;
            public FloatField Armature;
            public Label ExclusivityWarning;
            public Label VelocityGainWarning;
        }

        // SI units of the quantities an actuator on one joint type
        // touches. On a joint transmission the actuator's length is
        // gear[0]·qpos, so with gear[0] = 1 (every authored actuator) the
        // control, kp, and range units are the joint's own: a slide
        // works in m, m/s, N; a hinge in rad, rad/s, N·m. `None` (no
        // target, or gear[0] ≠ 1, where the length is scaled) shows no
        // joint-dependent unit rather than a wrong one. The three sets are
        // computed properties, built per call, so the type holds no static
        // state (Unity's lifecycle analyzer, UAL0010/UAL0013, flags static
        // fields and auto-properties).
        private sealed class Units
        {
            public static Units Slide => new Units
            {
                Position = "m",
                Velocity = "m/s",
                Force = "N",
                Stiffness = "N/m",
                Damping = "N·s/m",
                VelocityGain = "N/(m/s)",
                Inertia = "kg",
            };

            public static Units Hinge => new Units
            {
                Position = "rad",
                Velocity = "rad/s",
                Force = "N·m",
                Stiffness = "N·m/rad",
                Damping = "N·m·s/rad",
                VelocityGain = "N·m/(rad/s)",
                Inertia = "kg·m²",
            };

            public static Units None => new Units();

            public string Position = "";
            public string Velocity = "";
            public string Force = "";
            public string Stiffness = "";
            public string Damping = "";
            public string VelocityGain = "";
            public string Inertia = "";
        }

        // Re-applied on every property change (the type, the target, the
        // gear, the values), so the form follows at once. Hidden fields
        // keep their stored values; the exporter writes only the selected
        // type's attributes. The range fields are their limit switch's
        // payload: with the switch off they are inert, so they grey out
        // (the joint form's pattern). The exclusivity warning covers
        // <position>'s exclusive pairs: kv vs dampratio, inheritrange vs
        // an authored ctrlrange. The gain warning covers a <velocity>
        // with kv 0, which produces no force (the field defaults to
        // <position>'s 0, not <velocity>'s 1).
        private void UpdateForm(Form form)
        {
            SerializedProperty type = serializedObject.FindProperty("mj.type");
            bool isPosition = type == null || type.enumValueIndex == (int)StepActuatorType.Position;
            bool isVelocity = type != null && type.enumValueIndex == (int)StepActuatorType.Velocity;
            Show(form.Kp, isPosition);
            Show(form.Dampratio, isPosition);
            Show(form.Timeconst, isPosition);
            Show(form.Inheritrange, isPosition);

            Units units = UnitsOf(serializedObject);
            SetLabel(form.Delay, "Delay", "s");
            SetLabel(form.CtrlLo, "Ctrl Lo", isVelocity ? units.Velocity : units.Position);
            SetLabel(form.CtrlHi, "Ctrl Hi", isVelocity ? units.Velocity : units.Position);
            SetLabel(form.ForceLo, "Force Lo", units.Force);
            SetLabel(form.ForceHi, "Force Hi", units.Force);
            SetLabel(form.LengthrangeLo, "Lengthrange Lo", units.Position);
            SetLabel(form.LengthrangeHi, "Lengthrange Hi", units.Position);
            SetLabel(form.Cranklength, "Cranklength", "m");
            SetLabel(form.Kp, "Kp", units.Stiffness);
            SetLabel(form.Kv, isVelocity ? "Kv gain" : "Kv damping", isVelocity ? units.VelocityGain : units.Damping);
            SetLabel(form.Dampratio, "Dampratio", "1 = critical");
            SetLabel(form.Timeconst, "Timeconst", "s");
            SetLabel(form.Inheritrange, "Inheritrange", "× joint range");
            SetLabel(form.Damping, "Damping", units.Damping);
            SetLabel(form.Armature, "Armature", units.Inertia);

            SerializedProperty ctrl = serializedObject.FindProperty("mj.ctrllimited");
            SerializedProperty force = serializedObject.FindProperty("mj.forcelimited");
            bool ctrlOn = ctrl != null && ctrl.boolValue;
            bool forceOn = force != null && force.boolValue;
            form.CtrlLo?.SetEnabled(ctrlOn);
            form.CtrlHi?.SetEnabled(ctrlOn);
            form.ForceLo?.SetEnabled(forceOn);
            form.ForceHi?.SetEnabled(forceOn);

            SerializedProperty kv = serializedObject.FindProperty("mj.kv");
            SerializedProperty dampratio = serializedObject.FindProperty("mj.dampratio");
            SerializedProperty inheritrange = serializedObject.FindProperty("mj.inheritrange");
            bool conflict = (kv != null && kv.floatValue != 0f && dampratio != null && dampratio.floatValue != 0f)
                || (inheritrange != null && inheritrange.floatValue != 0f && ctrlOn);
            Show(form.ExclusivityWarning, isPosition && conflict);
            Show(form.VelocityGainWarning, isVelocity && kv != null && kv.floatValue == 0f);
        }

        // The target joint's units, or None when there is no target or
        // gear[0] ≠ 1 (see Units).
        private static Units UnitsOf(SerializedObject actuator)
        {
            SerializedProperty target = actuator.FindProperty("target");
            if (target == null || !(target.objectReferenceValue is StepJoint joint))
            {
                return Units.None;
            }
            SerializedProperty gear = actuator.FindProperty("mj.gear");
            if (gear != null && gear.isArray && gear.arraySize > 0 && gear.GetArrayElementAtIndex(0).floatValue != 1f)
            {
                return Units.None;
            }
            return joint.mj.type == StepJointType.Hinge ? Units.Hinge : Units.Slide;
        }

        private static void SetLabel(FloatField field, string name, string unit)
        {
            if (field != null)
            {
                field.label = string.IsNullOrEmpty(unit) ? name : $"{name} ({unit})";
            }
        }

        private static void Show(VisualElement element, bool visible)
        {
            if (element != null)
            {
                element.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None;
            }
        }
    }
}
