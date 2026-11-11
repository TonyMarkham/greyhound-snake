using System.Collections.Generic;
using System.Linq;
using UnityEditor;
using UnityEngine;

using SO = Greyhound.MuJoCo.ScriptableObjects;

#if UNITY_6000_5_OR_NEWER
using Unity.Scripting.LifecycleManagement;
#endif

namespace Greyhound.MuJoCo
{
    // Hierarchy feedback, duplicated from the step package's
    // StepJointHierarchyMarker: rows of GameObjects that a Joint targets
    // get a faint tint and a marker bar, so jointed bodies are visible
    // without selecting anything. Nothing is stored on the GameObjects.
    //
    // Membership: for a row's GameObject, the nearest ancestor-or-self
    // that is the instance root of an asset gives that asset and the
    // GameObject's indices below it ([SELF] for the root itself). The row
    // is marked when any Root whose model (Root.root) is that asset has a
    // joint whose body has the same root and the same indices.
    //
    // The Root assets are found once and cached; the cache is dropped
    // whenever the project changes (an asset added, removed or edited),
    // so a new Root or joint shows on the next repaint.
    [InitializeOnLoad]
#if UNITY_6000_5_OR_NEWER
    [NoAutoStaticsCleanup]
#endif
    internal static class JointHierarchyMarker
    {
        private static readonly Color RowTint = new(1.0f, 0.55f, 0.2f, 0.08f);
        private static readonly Color MarkerBar = new(1.0f, 0.55f, 0.2f, 0.9f);

        private static List<SO.Root> s_roots;

        static JointHierarchyMarker()
        {
#if UNITY_6000_4_OR_NEWER
            EditorApplication.hierarchyWindowItemByEntityIdOnGUI += OnHierarchyGUIByEntityId;
#else
            EditorApplication.hierarchyWindowItemOnGUI += OnHierarchyGUI;
#endif
            EditorApplication.projectChanged += () => s_roots = null;
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
            if (node == null || !HasJoint(node.transform))
            {
                return;
            }
            EditorGUI.DrawRect(selectionRect, RowTint);
            var bar = new Rect(selectionRect.x, selectionRect.y, 3f, selectionRect.height);
            EditorGUI.DrawRect(bar, MarkerBar);
        }

        private static bool HasJoint(Transform node)
        {
            if (!SiblingPathScene.TryFindInstanceRoot(node, out Transform instanceRoot, out GameObject rootAsset))
            {
                return false;
            }
            int[] indices = null;
            foreach (SO.Root root in Roots())
            {
                if (root == null || root.root != rootAsset || root.joints == null)
                {
                    continue;
                }
                indices ??= SiblingPathScene.IndicesOf(node, instanceRoot);
                foreach (SO.Joint joint in root.joints)
                {
                    SiblingPath body = joint != null ? joint.body : null;
                    if (body != null && body.root == rootAsset && body.indices != null
                        && body.indices.SequenceEqual(indices))
                    {
                        return true;
                    }
                }
            }
            return false;
        }

        private static List<SO.Root> Roots()
        {
            if (s_roots == null)
            {
                s_roots = AssetDatabase.FindAssets("t:" + nameof(SO.Root))
                    .Select(AssetDatabase.GUIDToAssetPath)
                    .Select(AssetDatabase.LoadAssetAtPath<SO.Root>)
                    .Where(root => root != null)
                    .ToList();
            }
            return s_roots;
        }
    }
}
