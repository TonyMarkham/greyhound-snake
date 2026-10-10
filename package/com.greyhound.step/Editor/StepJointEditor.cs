using UnityEditor;
using UnityEditor.UIElements;

using Unity.Scripting.LifecycleManagement;

using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The joint asset's inspector: a bound UXML form plus the scene
    // tools drawn through the static SceneView.duringSceneGui callback
    // (Handles are not UXML — only the forms are; Unity 6.7 removed the
    // per-editor OnSceneGUI hook). The UI assets are serialized
    // references assigned on this script asset — package-relative Style
    // src is unreliable, and the GUID in the shipped .meta keeps the
    // assignments stable. Scene tools: the axis-pick rays (auto-entered
    // on creation from the menu, re-entered from the form's button), the
    // hinge position handle, and the travel gizmo, drawn while the joint
    // is selected. Validation covers the cheap field-level rules:
    // nonzero axis and lo < hi when limits are on.
    [CustomEditor(typeof(StepJoint))]
    [NoAutoStaticsCleanup]
    internal sealed class StepJointEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        // Pick session state is static: exactly one SceneView exists,
        // and the creation menu must be able to auto-enter a session for
        // a joint whose editor instance may not exist yet. The saved
        // tool is restored when the session ends (click, Esc, or the
        // editor closing).
        private static StepJoint s_pickTarget;
        private static Tool s_savedTool = Tool.Move;

        // The joint whose scene tools are active. Selection cannot be
        // the gate: a locked inspector keeps its editor alive while the
        // selection moves elsewhere, and the gizmo must keep matching
        // what that locked inspector shows. The static claim therefore
        // follows the editor lifecycle instead: any joint editor opening
        // claims it; the claim clears when that editor dies while the
        // joint is not selected (an unlocked inspector's lifecycle, or a
        // closed window), when another joint's editor takes over, or
        // when the asset is deleted.
        private static StepJoint s_activeJoint;

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

            Button repick = root.Q<Button>("repick-axis");
            if (repick != null)
            {
                repick.clicked += BeginPickFromButton;
            }

            Label axisWarning = root.Q<Label>("axis-warning");
            Label limitsWarning = root.Q<Label>("limits-warning");
            UpdateValidation(axisWarning, limitsWarning);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                UpdateValidation(axisWarning, limitsWarning));
            return root;
        }

        private void OnEnable()
        {
            SceneView.duringSceneGui += OnSceneGui;
            s_activeJoint = (StepJoint)target;
        }

        private void OnDisable()
        {
            SceneView.duringSceneGui -= OnSceneGui;
            if (s_pickTarget == target)
            {
                EndPick();
            }
            if (s_activeJoint == target && Selection.activeObject != target)
            {
                s_activeJoint = null;
            }
        }

        // RepaintAll issued inside another window's UIElements event
        // dispatch is swallowed (same Unity 6.7 quirk the creation menu
        // defers around), so the session starts from a delayCall tick.
        // The joint must also be the active selection for the draw gate
        // to pass — a locked inspector is not selection — so the click
        // selects the asset first when it is not already selected.
        private void BeginPickFromButton()
        {
            var joint = (StepJoint)target;
            EditorApplication.delayCall += () =>
            {
                if (Selection.activeObject != joint)
                {
                    Selection.activeObject = joint;
                }
                BeginPick(joint);
            };
        }

        // Auto-enter hook for the creation menu: the session state is
        // static, so it starts even before this joint's editor instance
        // exists; the subscribed OnSceneGui picks it up on the next repaint.
        internal static void BeginPick(StepJoint joint)
        {
            if (joint == null || s_pickTarget == joint)
            {
                return;
            }
            if (s_pickTarget == null)
            {
                s_savedTool = Tools.current;
            }
            s_pickTarget = joint;
            Tools.current = Tool.None;
            SceneView.RepaintAll();
        }

        private static void EndPick()
        {
            s_pickTarget = null;
            Tools.current = s_savedTool;
            SceneView.RepaintAll();
        }

        private void UpdateValidation(Label axisWarning, Label limitsWarning)
        {
            var joint = (StepJoint)target;
            if (joint == null)
            {
                return;
            }
            axisWarning.style.display =
                joint.mj.axis.sqrMagnitude < 0.5f ? DisplayStyle.Flex : DisplayStyle.None;
            limitsWarning.style.display =
                joint.mj.rangeEnabled && joint.mj.rangeLo >= joint.mj.rangeHi
                    ? DisplayStyle.Flex
                    : DisplayStyle.None;
        }

        // Instance subscription keeps one callback per live editor; the
        // gate draws only for the joint that owns the tools
        // (s_activeJoint), which covers the selected and the locked-
        // inspector cases alike.
        private void OnSceneGui(SceneView sceneView)
        {
            // A deleted asset leaves a fake-null claim behind; clear it
            // so a future editor can take the slot.
            if (!ReferenceEquals(s_activeJoint, null) && s_activeJoint == null)
            {
                s_activeJoint = null;
            }
            var joint = (StepJoint)target;
            if (joint == null || joint != s_activeJoint)
            {
                return;
            }
            GameObject body = StepJointSetFactory.FindPart(joint);
            if (body == null)
            {
                return;
            }
            Matrix4x4 localToWorld = body.transform.localToWorldMatrix;
            Matrix4x4 worldToLocal = body.transform.worldToLocalMatrix;

            Event sceneEvent = Event.current;
            if (s_pickTarget == joint)
            {
                if (sceneEvent.type == EventType.KeyDown && sceneEvent.keyCode == KeyCode.Escape)
                {
                    sceneEvent.Use();
                    EndPick();
                    return;
                }
                DrawPickRays(joint, localToWorld, worldToLocal);
                Handles.Label(
                    localToWorld.MultiplyPoint3x4(Vector3.zero),
                    $"{joint.name}: click a ray to set the axis — Esc cancels");
                return;
            }

            DrawGizmos(joint, localToWorld);
            DrawPositionHandle(joint, localToWorld, worldToLocal);
        }

        private static void DrawPickRays(
            StepJoint joint, Matrix4x4 localToWorld, Matrix4x4 worldToLocal)
        {
            Vector3 origin = localToWorld.MultiplyPoint3x4(Vector3.zero);
            var directions = new[]
            {
                Vector3.right, Vector3.left, Vector3.up, Vector3.down, Vector3.forward, Vector3.back,
            };
            float rayLength = HandleUtility.GetHandleSize(origin) * 2.0f;
            Handles.color = new Color(1.0f, 0.5f, 0.0f);
            foreach (Vector3 localDirection in directions)
            {
                Vector3 worldDirection = localToWorld.MultiplyVector(localDirection).normalized;
                Vector3 tip = origin + worldDirection * rayLength;
                Handles.DrawLine(origin, tip);
                if (Handles.Button(
                        tip,
                        Quaternion.LookRotation(worldDirection),
                        HandleUtility.GetHandleSize(tip) * 0.25f,
                        HandleUtility.GetHandleSize(tip) * 0.5f,
                        Handles.SphereHandleCap))
                {
                    SetAxis(joint, worldToLocal, worldDirection);
                    return;
                }
            }
        }

        private static void SetAxis(StepJoint joint, Matrix4x4 worldToLocal, Vector3 worldDirection)
        {
            Vector3 local = worldToLocal.MultiplyVector(worldDirection);
            if (local.sqrMagnitude > 1e-10f)
            {
                Undo.RecordObject(joint, "Set Joint Axis");
                joint.mj.axis = local.normalized;
                EditorUtility.SetDirty(joint);
            }
            EndPick();
        }

        private static void DrawGizmos(StepJoint joint, Matrix4x4 localToWorld)
        {
            Vector3 axisWorld = localToWorld.MultiplyVector(joint.mj.axis);
            if (axisWorld.sqrMagnitude < 1e-10f)
            {
                return;
            }
            axisWorld.Normalize();
            Vector3 posWorld = localToWorld.MultiplyPoint3x4(joint.mj.pos);
            float arm = HandleUtility.GetHandleSize(posWorld) * 2.0f;

            Handles.color = Color.cyan;
            Handles.DrawLine(posWorld - axisWorld * arm, posWorld + axisWorld * arm);

            Handles.color = Color.yellow;
            if (joint.mj.type == StepJointType.Hinge)
            {
                // Orthonormal frame perpendicular to the axis; the arc's
                // zero reference is arbitrary (MJCF measures the range
                // against the body's initial orientation) — the span is
                // the display content.
                Vector3 u = Vector3.Cross(axisWorld, Vector3.up);
                if (u.sqrMagnitude < 1e-6f)
                {
                    u = Vector3.Cross(axisWorld, Vector3.right);
                }
                u.Normalize();
                if (joint.mj.rangeEnabled)
                {
                    Vector3 from = Quaternion.AngleAxis(
                        joint.mj.rangeLo * Mathf.Rad2Deg, axisWorld) * u;
                    float sweep = (joint.mj.rangeHi - joint.mj.rangeLo) * Mathf.Rad2Deg;
                    Handles.DrawWireArc(posWorld, axisWorld, from, sweep, arm);
                }
                else
                {
                    Handles.DrawWireArc(posWorld, axisWorld, u, 360.0f, arm);
                }
            }
            else if (joint.mj.rangeEnabled)
            {
                Vector3 loPos = posWorld + axisWorld * joint.mj.rangeLo;
                Vector3 hiPos = posWorld + axisWorld * joint.mj.rangeHi;
                Handles.DrawLine(loPos, hiPos);
                float cap = arm * 0.05f;
                Handles.DrawLine(loPos - axisWorld * cap, loPos + axisWorld * cap);
                Handles.DrawLine(hiPos - axisWorld * cap, hiPos + axisWorld * cap);
            }
        }

        private static void DrawPositionHandle(
            StepJoint joint, Matrix4x4 localToWorld, Matrix4x4 worldToLocal)
        {
            if (joint.mj.type != StepJointType.Hinge)
            {
                return;
            }
            Vector3 posWorld = localToWorld.MultiplyPoint3x4(joint.mj.pos);
            EditorGUI.BeginChangeCheck();
            // Handle axes follow the body frame, so aligned drags stay
            // aligned with the authored pos/axis coordinates.
            Vector3 moved = Handles.PositionHandle(posWorld, localToWorld.rotation);
            if (EditorGUI.EndChangeCheck())
            {
                Undo.RecordObject(joint, "Move Joint Position");
                joint.mj.pos = worldToLocal.MultiplyPoint3x4(moved);
                EditorUtility.SetDirty(joint);
            }
        }
    }
}
