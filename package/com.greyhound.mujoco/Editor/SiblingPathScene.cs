using System.Collections.Generic;

using UnityEditor;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Greyhound.MuJoCo
{
    // Finds what a SiblingPath names in the open scenes, for the editor
    // tools (the SiblingPath drawer, the joint and site scene tools, the
    // hierarchy marker).
    //
    // The instance of a root asset is the first GameObject, in hierarchy
    // order across the loaded scenes, that is a prefab instance root
    // whose original source is that asset. The target is that instance
    // for indices [SELF] = [-1], else the walk down the child indices;
    // empty indices name no target.
    //
    // The target's transform is the MJCF body frame its elements are
    // authored in: the exporter writes each GameObject as a <body> with
    // its localPosition and localRotation (no scale), so a joint's or
    // site's pos/axis/quat map to world space through the target's
    // localToWorldMatrix.
    internal static class SiblingPathScene
    {
        public static Transform FindInstance(GameObject rootAsset)
        {
            if (rootAsset == null)
            {
                return null;
            }
            for (int i = 0; i < SceneManager.sceneCount; i++)
            {
                foreach (GameObject sceneRoot in SceneManager.GetSceneAt(i).GetRootGameObjects())
                {
                    Transform found = FindInstanceBelow(sceneRoot.transform, rootAsset);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
            return null;
        }

        // The path's target in the open scenes, or null (no root, no
        // target, no instance open, or a path that no longer fits).
        public static Transform Resolve(SiblingPath path)
        {
            if (path == null || path.root == null || path.indices == null || path.indices.Length == 0)
            {
                return null;
            }
            Transform node = FindInstance(path.root);
            if (node == null || IsSelf(path.indices))
            {
                return node;
            }
            foreach (int index in path.indices)
            {
                if (index < 0 || index >= node.childCount)
                {
                    return null;
                }
                node = node.GetChild(index);
            }
            return node;
        }

        // The nearest ancestor-or-self of a scene GameObject that is the
        // instance root of an asset (its original source is a main
        // asset), and that asset. False when there is none.
        public static bool TryFindInstanceRoot(Transform node, out Transform instanceRoot, out GameObject rootAsset)
        {
            for (Transform candidate = node; candidate != null; candidate = candidate.parent)
            {
                GameObject source = PrefabUtility.GetCorrespondingObjectFromOriginalSource(candidate.gameObject);
                if (source != null && AssetDatabase.IsMainAsset(source))
                {
                    instanceRoot = candidate;
                    rootAsset = source;
                    return true;
                }
            }
            instanceRoot = null;
            rootAsset = null;
            return false;
        }

        // The indices of `node` below `instanceRoot`: [SELF] for the root
        // itself, else the child indices. Example: root -> A (index 1) ->
        // B (index 0) gives [1, 0].
        public static int[] IndicesOf(Transform node, Transform instanceRoot)
        {
            if (node == instanceRoot)
            {
                return new[] { SiblingPath.SELF };
            }
            var reversed = new List<int>();
            for (Transform step = node; step != null && step != instanceRoot; step = step.parent)
            {
                reversed.Add(step.GetSiblingIndex());
            }
            reversed.Reverse();
            return reversed.ToArray();
        }

        public static bool IsSelf(int[] indices)
        {
            return indices != null && indices.Length == 1 && indices[0] == SiblingPath.SELF;
        }

        private static Transform FindInstanceBelow(Transform node, GameObject rootAsset)
        {
            if (PrefabUtility.IsAnyPrefabInstanceRoot(node.gameObject) &&
                PrefabUtility.GetCorrespondingObjectFromOriginalSource(node.gameObject) == rootAsset)
            {
                return node;
            }
            foreach (Transform child in node)
            {
                Transform found = FindInstanceBelow(child, rootAsset);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }
    }
}
