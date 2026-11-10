using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Sensor inspector. Layout in UI/Sensor.uxml, styles in
    // UI/Sensor.uss, both assigned as this script's default references
    // (its .meta), as for JointEditor. Top to bottom: the identity card
    // (name) with the Sensors bar colour, Measures (objtype, objname,
    // reftype, refname) and Signal (noise, cutoff) open, then Timing and
    // Other collapsed. Labels are the MJCF attribute names; an empty
    // refname means world coordinates. The asset is renamed to match the MJCF name when
    // one is set (ElementEditor).
    [CustomEditor(typeof(SO.Sensor))]
    internal sealed class SensorEditor : ElementEditor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        protected override string NamePath => "sensor.name";

        protected override VisualElement BuildLayout()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(SensorEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(SensorEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            // Only the objname/refname field matching its type is shown
            // (Site, Geom, or Body for both Body and XBody); the others keep
            // their values but are not exported. Camera has no field and
            // shows a note instead.
            Label cameraNote = root.Q<Label>("camera-note");
            SerializedProperty objtype = serializedObject.FindProperty("objtype");
            SerializedProperty reftype = serializedObject.FindProperty("reftype");
            void Refresh(SerializedProperty _)
            {
                var obj = (FrameObject)objtype.enumValueIndex;
                var reference = (FrameObject)reftype.enumValueIndex;
                ShowForType(root, "obj", obj);
                ShowForType(root, "ref", reference);
                bool camera = obj == FrameObject.Camera || reference == FrameObject.Camera;
                cameraNote.style.display = camera ? DisplayStyle.Flex : DisplayStyle.None;
            }
            Refresh(objtype);
            root.TrackPropertyValue(objtype, Refresh);
            root.TrackPropertyValue(reftype, Refresh);
            return root;
        }

        private static void ShowForType(VisualElement root, string prefix, FrameObject type)
        {
            Show(root, $"{prefix}-site", type == FrameObject.Site);
            Show(root, $"{prefix}-geom", type == FrameObject.Geom);
            Show(root, $"{prefix}-body", type == FrameObject.Body || type == FrameObject.XBody);
        }

        private static void Show(VisualElement root, string className, bool visible)
        {
            root.Query<VisualElement>(className: className)
                .ForEach(field => field.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None);
        }
    }
}
