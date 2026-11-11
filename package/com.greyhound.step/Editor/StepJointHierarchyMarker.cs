using UnityEditor;
using UnityEngine;

namespace Greyhound.Step
{
    // Persistent Hierarchy feedback: rows belonging to jointed parts get
    // a faint tint and a marker bar, so annotated bodies are visible at
    // a glance without selecting anything. Membership resolves through
    // the containers (cached asset loads + path comparison) — nothing
    // is stored on the parts, and the tint travels with the annotation
    // across reimports by construction.
    [InitializeOnLoad]
    internal static class StepJointHierarchyMarker
    {
        private static readonly Color RowTint = new(1.0f, 0.55f, 0.2f, 0.08f);
        private static readonly Color MarkerBar = new(1.0f, 0.55f, 0.2f, 0.9f);

        static StepJointHierarchyMarker()
        {
#if UNITY_6000_4_OR_NEWER
            EditorApplication.hierarchyWindowItemByEntityIdOnGUI += OnHierarchyGUIByEntityId;
#else
            EditorApplication.hierarchyWindowItemOnGUI += OnHierarchyGUI;
#endif
        }

        // Each Unity version gets the handler its hierarchy event expects:
        // 6000.4+ passes an EntityId; 6000.3 passes an int but has made
        // InstanceIDToObject obsolete, so the int is cast to an EntityId
        // (as Unity's shadergraph does); older versions look up the int.
#if UNITY_6000_4_OR_NEWER
        private static void OnHierarchyGUIByEntityId(EntityId entityId, Rect selectionRect)
        {
            DrawRow(EditorUtility.EntityIdToObject(entityId) as GameObject, selectionRect);
        }
#elif UNITY_6000_3_OR_NEWER
        private static void OnHierarchyGUI(int instanceId, Rect selectionRect)
        {
            DrawRow(EditorUtility.EntityIdToObject((EntityId)instanceId) as GameObject, selectionRect);
        }
#else
        private static void OnHierarchyGUI(int instanceId, Rect selectionRect)
        {
            DrawRow(EditorUtility.InstanceIDToObject(instanceId) as GameObject, selectionRect);
        }
#endif

        private static void DrawRow(GameObject node, Rect selectionRect)
        {
            if (node == null || !TryGetJoint(node, out StepJoint _))
            {
                return;
            }
            EditorGUI.DrawRect(selectionRect, RowTint);
            var bar = new Rect(selectionRect.x, selectionRect.y, 3f, selectionRect.height);
            EditorGUI.DrawRect(bar, MarkerBar);
        }

        private static bool TryGetJoint(GameObject node, out StepJoint joint)
        {
            joint = null;
            if (!StepJointSetFactory.TryResolveStepSource(
                    node, out GameObject instanceRoot, out string stepPath))
            {
                return false;
            }
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return false;
            }
            var joints = StepJointSetFactory.FindJoints(set, node, instanceRoot);
            if (joints.Count == 0)
            {
                return false;
            }
            joint = joints[0];
            return true;
        }
    }
}
