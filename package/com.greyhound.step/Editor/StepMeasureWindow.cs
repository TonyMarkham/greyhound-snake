using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // Measures the world-space offset between two scene objects: drop
    // them into From and To, press Measure, and the three fields show
    // To's world position minus From's, per world axis, in meters
    // (Unity's world frame, y up). Worked example: From at (1, 2, 3) and
    // To at (1.5, 2, 2) read 0.5, 0, -1. The layout is a UXML form; the
    // UI assets are serialized references assigned on this script asset
    // (the joint-family editor pattern). The fields' behavior is set
    // here: From and To take scene GameObjects, and the results are
    // read-only text, so a value can be selected and copied (Ctrl+C)
    // but not edited.
    internal sealed class StepMeasureWindow : EditorWindow
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        [MenuItem("Tools/Step/Measure")]
        private static void Open()
        {
            GetWindow<StepMeasureWindow>("Measure");
        }

        private void CreateGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                rootVisualElement.Add(new Label($"[{GetType().Namespace}::{nameof(StepMeasureWindow)}]: VisualTreeAsset Not Set"));
                return;
            }
            if (m_StyleSheet == null)
            {
                rootVisualElement.Add(new Label($"[{GetType().Namespace}::{nameof(StepMeasureWindow)}]: StyleSheet Not Set"));
                return;
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();
            root.styleSheets.Add(m_StyleSheet);
            rootVisualElement.Add(root);

            ObjectField from = SceneObjectField(root, "from");
            ObjectField to = SceneObjectField(root, "to");
            FloatField deltaX = ResultField(root, "delta-x");
            FloatField deltaY = ResultField(root, "delta-y");
            FloatField deltaZ = ResultField(root, "delta-z");

            root.Q<Button>("measure-button").clicked += () => Measure(from, to, deltaX, deltaY, deltaZ);
        }

        private static ObjectField SceneObjectField(VisualElement root, string name)
        {
            ObjectField field = root.Q<ObjectField>(name);
            field.objectType = typeof(GameObject);
            field.allowSceneObjects = true;
            return field;
        }

        private static FloatField ResultField(VisualElement root, string name)
        {
            FloatField field = root.Q<FloatField>(name);
            field.isReadOnly = true;
            return field;
        }

        private static void Measure(
            ObjectField from, ObjectField to, FloatField deltaX, FloatField deltaY, FloatField deltaZ)
        {
            if (from.value is not GameObject start || to.value is not GameObject end)
            {
                Debug.LogWarning("[StepMeasureWindow] Drop a scene object into both From and To.");
                return;
            }
            Vector3 delta = end.transform.position - start.transform.position;
            deltaX.SetValueWithoutNotify(delta.x);
            deltaY.SetValueWithoutNotify(delta.y);
            deltaZ.SetValueWithoutNotify(delta.z);
        }
    }
}
