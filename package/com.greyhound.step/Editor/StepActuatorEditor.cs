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
    // mirror: drive numbers are in joint units (meters for a slide
    // target, radians for a hinge).
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

            PropertyField ctrlLo = root.Q<PropertyField>("ctrl-lo");
            PropertyField ctrlHi = root.Q<PropertyField>("ctrl-hi");
            PropertyField forceLo = root.Q<PropertyField>("force-lo");
            PropertyField forceHi = root.Q<PropertyField>("force-hi");
            Label exclusivityWarning = root.Q<Label>("exclusivity-warning");
            UpdateSwitches(ctrlLo, ctrlHi, forceLo, forceHi, exclusivityWarning);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                UpdateSwitches(ctrlLo, ctrlHi, forceLo, forceHi, exclusivityWarning));
            return root;
        }

        // The range fields are their limit switch's payload: with the
        // switch off they are inert, so they grey out (the joint form's
        // pattern). The exclusivity warning covers the spec's
        // exclusive pairs: kv vs dampratio, inheritrange vs an authored
        // ctrlrange.
        private void UpdateSwitches(
            PropertyField ctrlLo, PropertyField ctrlHi, PropertyField forceLo, PropertyField forceHi,
            Label exclusivityWarning)
        {
            SerializedProperty ctrl = serializedObject.FindProperty("mj.ctrllimited");
            SerializedProperty force = serializedObject.FindProperty("mj.forcelimited");
            bool ctrlOn = ctrl != null && ctrl.boolValue;
            bool forceOn = force != null && force.boolValue;
            ctrlLo?.SetEnabled(ctrlOn);
            ctrlHi?.SetEnabled(ctrlOn);
            forceLo?.SetEnabled(forceOn);
            forceHi?.SetEnabled(forceOn);

            SerializedProperty kv = serializedObject.FindProperty("mj.kv");
            SerializedProperty dampratio = serializedObject.FindProperty("mj.dampratio");
            SerializedProperty inheritrange = serializedObject.FindProperty("mj.inheritrange");
            bool conflict = (kv != null && kv.floatValue != 0f && dampratio != null && dampratio.floatValue != 0f)
                || (inheritrange != null && inheritrange.floatValue != 0f && ctrlOn);
            if (exclusivityWarning != null)
            {
                exclusivityWarning.style.display =
                    conflict ? DisplayStyle.Flex : DisplayStyle.None;
            }
        }
    }
}
