using UnityEditor;
using UnityEditor.UIElements;

using Unity.Scripting.LifecycleManagement;

using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Joint inspector. Layout in UI/Joint.uxml, styles in
    // UI/Joint.uss, both assigned as this script's default references
    // (its .meta), as for RootEditor. Top to bottom: the identity card
    // (name, body) with the Joints bar colour, Joint and Range
    // (open), then Dynamics, Solver, Actuator force and Other
    // (collapsed). Labels are the MJCF attribute names.
    //
    // Scene tools, duplicated from the step package's StepJointEditor and
    // drawn through SceneView.duringSceneGui (Unity 6.7 removed the
    // per-editor OnSceneGUI hook), while this joint's editor is the
    // active one (selected, or shown in a locked inspector):
    //   - axis pick: six rays from the target's origin; click one to set
    //     `axis`, Esc cancels. Started by the form's Re-pick Axis button.
    //   - travel gizmo: the axis line (cyan) through pos, and the range
    //     (yellow): an arc for a hinge, a capped line for a slide; a full
    //     circle for an unlimited hinge. Ball and free joints have no
    //     single travel axis and draw none.
    //   - position handle for pos, on hinge and ball joints (a slide's pos
    //     is render-only in MJCF; a free joint has none).
    // Frame: MJCF authors pos and axis in the frame of the body the joint
    // is defined in. That body is the GameObject `body` resolves to in
    // the open scenes (SiblingPathScene), exported with its local pose,
    // so its localToWorldMatrix maps them to world space.
    //
    // Limits are on when `limited` is True, or Auto with a range set
    // (compiler autolimits); the range fields grey out only for False.
    // Warnings: a zero axis, and lo >= hi while limits are on.
    //
    // Naming: whenever the body path changes and has a root and a target,
    // an empty MJCF `name` becomes "j-[<indices>]-<targetName>", indices
    // comma-separated. Example: a part named "Pole" at [2,0] gives
    // "j-[2,0]-Pole"; the model root itself (indices [SELF] = [-1]) gives
    // "j-[-1]-<name>". A name already set is never replaced. Setting the
    // name renames the asset to match (ElementEditor). The exporter turns
    // [ , ] into _, so "j-[2,0]-Pole" exports as "j-_2_0_-Pole".
    [CustomEditor(typeof(SO.Joint))]
#if UNITY_6000_5_OR_NEWER
    [NoAutoStaticsCleanup]
#endif
    internal sealed class JointEditor : ElementEditor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        // Pick session state is static: exactly one SceneView draws the
        // session. The saved tool is restored when the session ends
        // (click, Esc, or the editor closing).
        private static SO.Joint s_pickTarget;
        private static Tool s_savedTool = Tool.Move;

        // The joint whose scene tools are active. Selection cannot be the
        // gate: a locked inspector keeps its editor alive while the
        // selection moves elsewhere. The claim follows the editor
        // lifecycle: any joint editor opening claims it; it clears when
        // that editor dies while the joint is not selected, when another
        // joint's editor takes over, or when the asset is deleted.
        private static SO.Joint s_activeJoint;

        protected override string NamePath => "joint.name";

        protected override VisualElement BuildLayout()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(JointEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(JointEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            SerializedProperty type = serializedObject.FindProperty("joint.type");
            void RefreshUnits(SerializedProperty property)
            {
                WriteUnits(root, (JointType)property.enumValueIndex);
            }
            RefreshUnits(type);
            root.TrackPropertyValue(type, RefreshUnits);

            Button repick = root.Q<Button>("repick-axis");
            if (repick != null)
            {
                repick.clicked += BeginPickFromButton;
            }
            UpdateValidation(root);
            root.RegisterCallback<SerializedPropertyChangeEvent>(_ => UpdateValidation(root));
            return root;
        }

        protected override void Extend(VisualElement container)
        {
            container.TrackPropertyValue(
                serializedObject.FindProperty("body"), _ => FillName("j-", ((SO.Joint)target).body));
        }

        private void OnEnable()
        {
            SceneView.duringSceneGui += OnSceneGui;
            s_activeJoint = (SO.Joint)target;
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

        // SI units by joint type (the repo's convention; exports write
        // angle="radian"). A slide's coordinate is a length (m), so its
        // ranges, ref, springref and margin are m, forces N, stiffness
        // N/m, damping N·s/m, armature kg. A hinge's and a ball's are
        // angles (rad): torques N·m, stiffness N·m/rad, damping
        // N·m·s/rad, armature kg·m². A free joint has no single
        // coordinate, so those units are left off. damping and stiffness
        // are polynomials; the unit is the first coefficient's. pos (m)
        // and springdamper (s, 1 = critical) are fixed in the UXML.
        private static void WriteUnits(VisualElement root, JointType type)
        {
            bool linear = type == JointType.Slide;
            bool angular = type == JointType.Hinge || type == JointType.Ball;
            string Pick(string slide, string hinge) => linear ? slide : angular ? hinge : "";
            string coordinate = Pick("m", "rad");
            string force = Pick("N", "N·m");
            SetLabel(root, "unit-ref", "ref", coordinate);
            SetLabel(root, "unit-rangeLo", "range lo", coordinate);
            SetLabel(root, "unit-rangeHi", "range hi", coordinate);
            SetLabel(root, "unit-springref", "springref", coordinate);
            SetLabel(root, "unit-margin", "margin", coordinate);
            SetLabel(root, "unit-damping", "damping", Pick("N·s/m", "N·m·s/rad"));
            SetLabel(root, "unit-stiffness", "stiffness", Pick("N/m", "N·m/rad"));
            SetLabel(root, "unit-frictionloss", "frictionloss", force);
            SetLabel(root, "unit-armature", "armature", Pick("kg", "kg·m²"));
            SetLabel(root, "unit-actuatorfrcrangeLo", "actuatorfrcrange lo", force);
            SetLabel(root, "unit-actuatorfrcrangeHi", "actuatorfrcrange hi", force);
        }

        // Limits are on for limited = True, or Auto with a range set
        // (autolimits: a written range turns them on; the exporter writes
        // the range when either bound is nonzero).
        private static bool LimitsOn(Joint joint)
        {
            return joint.limited == FalseTrueAuto.True
                || (joint.limited == FalseTrueAuto.Auto && (joint.rangeLo != 0f || joint.rangeHi != 0f));
        }

        private void UpdateValidation(VisualElement root)
        {
            var asset = (SO.Joint)target;
            if (asset == null)
            {
                return;
            }
            Joint joint = asset.joint;
            Show(root.Q<Label>("axis-warning"), joint.axis.sqrMagnitude < 1e-10f);
            Show(root.Q<Label>("limits-warning"), LimitsOn(joint) && joint.rangeLo >= joint.rangeHi);
            // The range fields are inert only when limits are forced off.
            bool rangeEditable = joint.limited != FalseTrueAuto.False;
            root.Q<PropertyField>("unit-rangeLo")?.SetEnabled(rangeEditable);
            root.Q<PropertyField>("unit-rangeHi")?.SetEnabled(rangeEditable);
        }

        private static void Show(VisualElement element, bool visible)
        {
            if (element != null)
            {
                element.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None;
            }
        }

        // RepaintAll issued inside another window's UIElements event
        // dispatch is swallowed (a Unity 6.7 quirk), so the session starts
        // from a delayCall tick. The joint must be the active selection
        // for the draw gate to pass, so the click selects it first.
        private void BeginPickFromButton()
        {
            var joint = (SO.Joint)target;
            EditorApplication.delayCall += () =>
            {
                if (Selection.activeObject != joint)
                {
                    Selection.activeObject = joint;
                }
                BeginPick(joint);
            };
        }

        private static void BeginPick(SO.Joint joint)
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

        private void OnSceneGui(SceneView sceneView)
        {
            // A deleted asset leaves a fake-null claim behind; clear it so
            // a future editor can take the slot.
            if (!ReferenceEquals(s_activeJoint, null) && s_activeJoint == null)
            {
                s_activeJoint = null;
            }
            var asset = (SO.Joint)target;
            if (asset == null || asset != s_activeJoint)
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

            Event sceneEvent = Event.current;
            if (s_pickTarget == asset)
            {
                if (sceneEvent.type == EventType.KeyDown && sceneEvent.keyCode == KeyCode.Escape)
                {
                    sceneEvent.Use();
                    EndPick();
                    return;
                }
                DrawPickRays(asset, localToWorld, worldToLocal);
                Handles.Label(
                    localToWorld.MultiplyPoint3x4(Vector3.zero),
                    $"{asset.name}: click a ray to set the axis (Esc cancels)");
                return;
            }

            DrawGizmos(asset.joint, localToWorld);
            DrawPositionHandle(asset, localToWorld, worldToLocal);
        }

        private static void DrawPickRays(SO.Joint asset, Matrix4x4 localToWorld, Matrix4x4 worldToLocal)
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
                    SetAxis(asset, worldToLocal, worldDirection);
                    return;
                }
            }
        }

        private static void SetAxis(SO.Joint asset, Matrix4x4 worldToLocal, Vector3 worldDirection)
        {
            Vector3 local = worldToLocal.MultiplyVector(worldDirection);
            if (local.sqrMagnitude > 1e-10f)
            {
                Undo.RecordObject(asset, "Set Joint Axis");
                asset.joint.axis = local.normalized;
                EditorUtility.SetDirty(asset);
            }
            EndPick();
        }

        private static void DrawGizmos(Joint joint, Matrix4x4 localToWorld)
        {
            if (joint.type != JointType.Hinge && joint.type != JointType.Slide)
            {
                return;
            }
            Vector3 axisWorld = localToWorld.MultiplyVector(joint.axis);
            if (axisWorld.sqrMagnitude < 1e-10f)
            {
                return;
            }
            axisWorld.Normalize();
            Vector3 posWorld = localToWorld.MultiplyPoint3x4(joint.pos);
            float arm = HandleUtility.GetHandleSize(posWorld) * 2.0f;

            Handles.color = Color.cyan;
            Handles.DrawLine(posWorld - axisWorld * arm, posWorld + axisWorld * arm);

            Handles.color = Color.yellow;
            bool limits = LimitsOn(joint);
            if (joint.type == JointType.Hinge)
            {
                // Orthonormal frame perpendicular to the axis; the arc's
                // zero reference is arbitrary (MJCF measures the range
                // against the body's initial orientation): the span is the
                // display content.
                Vector3 u = Vector3.Cross(axisWorld, Vector3.up);
                if (u.sqrMagnitude < 1e-6f)
                {
                    u = Vector3.Cross(axisWorld, Vector3.right);
                }
                u.Normalize();
                if (limits)
                {
                    Vector3 from = Quaternion.AngleAxis(joint.rangeLo * Mathf.Rad2Deg, axisWorld) * u;
                    float sweep = (joint.rangeHi - joint.rangeLo) * Mathf.Rad2Deg;
                    Handles.DrawWireArc(posWorld, axisWorld, from, sweep, arm);
                }
                else
                {
                    Handles.DrawWireArc(posWorld, axisWorld, u, 360.0f, arm);
                }
            }
            else if (limits)
            {
                Vector3 loPos = posWorld + axisWorld * joint.rangeLo;
                Vector3 hiPos = posWorld + axisWorld * joint.rangeHi;
                Handles.DrawLine(loPos, hiPos);
                float cap = arm * 0.05f;
                Handles.DrawLine(loPos - axisWorld * cap, loPos + axisWorld * cap);
                Handles.DrawLine(hiPos - axisWorld * cap, hiPos + axisWorld * cap);
            }
        }

        private static void DrawPositionHandle(SO.Joint asset, Matrix4x4 localToWorld, Matrix4x4 worldToLocal)
        {
            JointType type = asset.joint.type;
            if (type != JointType.Hinge && type != JointType.Ball)
            {
                return;
            }
            Vector3 posWorld = localToWorld.MultiplyPoint3x4(asset.joint.pos);
            EditorGUI.BeginChangeCheck();
            // Handle axes follow the body frame, so aligned drags stay
            // aligned with the authored pos/axis coordinates.
            Vector3 moved = Handles.PositionHandle(posWorld, localToWorld.rotation);
            if (EditorGUI.EndChangeCheck())
            {
                Undo.RecordObject(asset, "Move Joint Position");
                asset.joint.pos = worldToLocal.MultiplyPoint3x4(moved);
                EditorUtility.SetDirty(asset);
            }
        }
    }
}
