using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The site asset's inspector: a bound UXML form over the MjSite
    // mirror, plus scene tools drawn through the static
    // SceneView.duringSceneGui callback (Unity 6.7 removed the
    // per-editor OnSceneGUI hook): a marker sphere, the site's name and
    // a position handle at the site, drawn while the site is the
    // selection. The handle's axes follow the part's frame, the frame
    // pos is authored in, as the hinge joint's position handle does.
    // The UI assets are serialized references assigned on this script
    // asset (the joint-family editor pattern).
    [CustomEditor(typeof(StepSite))]
    internal sealed class StepSiteEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepSiteEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepSiteEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);
            return root;
        }

        private void OnEnable()
        {
            SceneView.duringSceneGui += OnSceneGui;
        }

        private void OnDisable()
        {
            SceneView.duringSceneGui -= OnSceneGui;
        }

        private void OnSceneGui(SceneView sceneView)
        {
            var site = (StepSite)target;
            if (site == null || Selection.activeObject != site)
            {
                return;
            }
            GameObject body = StepJointSetFactory.FindPart(site.root, site.body);
            if (body == null)
            {
                return;
            }
            Matrix4x4 localToWorld = body.transform.localToWorldMatrix;
            Matrix4x4 worldToLocal = body.transform.worldToLocalMatrix;
            Vector3 posWorld = localToWorld.MultiplyPoint3x4(site.mj.pos);

            if (Event.current.type == EventType.Repaint)
            {
                Handles.color = Color.magenta;
                Handles.SphereHandleCap(
                    0, posWorld, Quaternion.identity,
                    HandleUtility.GetHandleSize(posWorld) * 0.15f, EventType.Repaint);
            }
            Handles.Label(posWorld, site.name);

            EditorGUI.BeginChangeCheck();
            Vector3 moved = Handles.PositionHandle(posWorld, localToWorld.rotation);
            if (EditorGUI.EndChangeCheck())
            {
                Undo.RecordObject(site, "Move Site Position");
                site.mj.pos = worldToLocal.MultiplyPoint3x4(moved);
                EditorUtility.SetDirty(site);
            }
        }
    }
}
