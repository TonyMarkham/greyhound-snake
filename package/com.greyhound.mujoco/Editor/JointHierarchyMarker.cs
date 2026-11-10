using System.Collections.Generic;
using System.Linq;

using UnityEditor;

using Unity.Scripting.LifecycleManagement;

using UnityEngine;

using SO = Greyhound.MuJoCo.ScriptableObjects;

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
    [NoAutoStaticsCleanup]
    internal static class JointHierarchyMarker
    {
        private static readonly Color RowTint = new(1.0f, 0.55f, 0.2f, 0.08f);
        private static readonly Color MarkerBar = new(1.0f, 0.55f, 0.2f, 0.9f);

        private static List<SO.Root> s_roots;

        static JointHierarchyMarker()
        {
            EditorApplication.hierarchyWindowItemByEntityIdOnGUI += OnHierarchyItem;
            EditorApplication.projectChanged += () => s_roots = null;
        }

        private static void OnHierarchyItem(EntityId entityId, Rect selectionRect)
        {
            GameObject node = EditorUtility.EntityIdToObject(entityId) as GameObject;
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
