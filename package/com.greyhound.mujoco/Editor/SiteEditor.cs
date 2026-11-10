using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Site inspector. Layout in UI/Site.uxml, styles in UI/Site.uss,
    // both assigned as this script's default references (its .meta), as
    // for JointEditor. Top to bottom: the identity card (name, body) with
    // the Sites bar colour, Placement and Shape (open), then Appearance
    // and Other (collapsed). Labels are the MJCF attribute names.
    //
    // Shown only when they apply (values stay stored either way):
    //   - pos, quat (class "frame-only") while useFromto is off: with
    //     fromto set, MuJoCo ignores them.
    //   - fromto from/to (class "fromto-only") while useFromto is on.
    //   - mesh (class "mesh-only") while type is Mesh.
    //
    // Naming: when the body path changes and has a root, an empty MJCF
    // `name` becomes "s-[<indices>]-<targetName>", e.g. "s-[2,0]-Pole". A
    // name already set is never replaced. Setting the name renames the
    // asset to match (ElementEditor).
    //
    // Scene tools, duplicated from the step package's StepSiteEditor and
    // drawn through SceneView.duringSceneGui while the site is the
    // selection: a magenta marker sphere at the site frame's origin, the
    // site's MJCF name, and a position handle for pos. Frame: MJCF
    // authors pos (and fromto) in the frame of the site's body, the
    // GameObject `body` resolves to in the open scenes (SiblingPathScene),
    // exported with its local pose, so its localToWorldMatrix maps them
    // to world space; the handle's axes follow that frame. With useFromto
    // on, MuJoCo ignores pos and puts the frame at the midpoint of
    // fromto: the marker sits there, the from-to segment is drawn, and
    // there is no position handle.
    [CustomEditor(typeof(SO.Site))]
    internal sealed class SiteEditor : ElementEditor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        protected override string NamePath => "site.name";

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
            var asset = (SO.Site)target;
            if (asset == null || Selection.activeObject != asset)
            {
                return;
            }
            Transform body = SiblingPathScene.Resolve(asset.body);
            if (body == null)
            {
                return;
            }
            Matrix4x4 localToWorld = body.localToWorldMatrix;
            Matrix4x4 worldToLocal = body.worldToLocalMatrix;
            Site site = asset.site;

            Vector3 originWorld;
            if (site.useFromto)
            {
                Vector3 fromWorld = localToWorld.MultiplyPoint3x4(site.fromtoFrom);
                Vector3 toWorld = localToWorld.MultiplyPoint3x4(site.fromtoTo);
                Handles.color = Color.magenta;
                Handles.DrawLine(fromWorld, toWorld);
                originWorld = (fromWorld + toWorld) * 0.5f;
            }
            else
            {
                originWorld = localToWorld.MultiplyPoint3x4(site.pos);
            }

            if (Event.current.type == EventType.Repaint)
            {
                Handles.color = Color.magenta;
                Handles.SphereHandleCap(
                    0, originWorld, Quaternion.identity,
                    HandleUtility.GetHandleSize(originWorld) * 0.15f, EventType.Repaint);
            }
            Handles.Label(originWorld, string.IsNullOrEmpty(site.name) ? asset.name : site.name);

            if (site.useFromto)
            {
                return;
            }
            EditorGUI.BeginChangeCheck();
            Vector3 moved = Handles.PositionHandle(originWorld, localToWorld.rotation);
            if (EditorGUI.EndChangeCheck())
            {
                Undo.RecordObject(asset, "Move Site Position");
                site.pos = worldToLocal.MultiplyPoint3x4(moved);
                EditorUtility.SetDirty(asset);
            }
        }

        protected override VisualElement BuildLayout()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(SiteEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(SiteEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            SerializedProperty useFromto = serializedObject.FindProperty("site.useFromto");
            void ShowForFromto(SerializedProperty property)
            {
                Show(root, "frame-only", !property.boolValue);
                Show(root, "fromto-only", property.boolValue);
            }
            ShowForFromto(useFromto);
            root.TrackPropertyValue(useFromto, ShowForFromto);

            SerializedProperty type = serializedObject.FindProperty("site.type");
            void ShowForType(SerializedProperty property)
            {
                Show(root, "mesh-only", property.enumValueIndex == (int)SiteType.Mesh);
            }
            ShowForType(type);
            root.TrackPropertyValue(type, ShowForType);
            return root;
        }

        protected override void Extend(VisualElement container)
        {
            container.TrackPropertyValue(
                serializedObject.FindProperty("body"), _ => FillName("s-", ((SO.Site)target).body));
        }

        private static void Show(VisualElement root, string className, bool visible)
        {
            root.Query<VisualElement>(className: className)
                .ForEach(field => field.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None);
        }
    }
}
