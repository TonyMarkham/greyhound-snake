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
            return root;
        }
    }
}
