using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The geom asset's inspector: a bound UXML form over the MjGeom
    // mirror — the full <geom> attribute surface in spec order. The UI
    // assets are serialized references assigned on this script asset
    // (the joint-family editor pattern; the shipped .meta GUIDs keep
    // the assignments stable). The form warns when more than one
    // orientation mechanism is authored — the spec allows at most one
    // per geom; the exporter normalizes to the canonical quat either
    // way.
    [CustomEditor(typeof(StepGeom))]
    internal sealed class StepGeomEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepGeomEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepGeomEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            Label orientationWarning = root.Q<Label>("orientation-warning");
            UpdateOrientationWarning(orientationWarning);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                UpdateOrientationWarning(orientationWarning));
            return root;
        }

        private void UpdateOrientationWarning(Label orientationWarning)
        {
            if (orientationWarning == null)
            {
                return;
            }
            int authored = 0;
            authored += IsArrayAuthored("mj.axisangle") ? 1 : 0;
            authored += IsArrayAuthored("mj.xyaxes") ? 1 : 0;
            authored += IsArrayAuthored("mj.zaxis") ? 1 : 0;
            authored += IsArrayAuthored("mj.euler") ? 1 : 0;
            orientationWarning.style.display =
                authored > 1 ? DisplayStyle.Flex : DisplayStyle.None;
        }

        private bool IsArrayAuthored(string propertyPath)
        {
            SerializedProperty array = serializedObject.FindProperty(propertyPath);
            return array != null && array.isArray && array.arraySize > 0;
        }
    }
}
