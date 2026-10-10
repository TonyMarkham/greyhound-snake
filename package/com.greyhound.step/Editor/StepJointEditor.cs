using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The joint asset's inspector: a bound UXML form. The UI assets are
    // serialized references assigned on this script asset — package-
    // relative Style src is unreliable, and the GUID in the shipped
    // .meta keeps the assignments stable. Scene-side authoring (axis
    // rays, origin handle, travel gizmo, wiggle) is M2c and binds to
    // this editor later; validation here covers the cheap field-level
    // rules: nonzero axis and lo < hi when limits are on.
    [CustomEditor(typeof(StepJoint))]
    internal sealed class StepJointEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepJointEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepJointEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            Label axisWarning = root.Q<Label>("axis-warning");
            Label limitsWarning = root.Q<Label>("limits-warning");
            UpdateValidation(axisWarning, limitsWarning);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                UpdateValidation(axisWarning, limitsWarning));
            return root;
        }

        private void UpdateValidation(Label axisWarning, Label limitsWarning)
        {
            var joint = (StepJoint)target;
            if (joint == null)
            {
                return;
            }
            axisWarning.style.display = joint.mj.axis.sqrMagnitude < 0.5f ? DisplayStyle.Flex : DisplayStyle.None;
            limitsWarning.style.display =
                joint.mj.rangeEnabled && joint.mj.rangeLo >= joint.mj.rangeHi ? DisplayStyle.Flex : DisplayStyle.None;
        }
    }
}
