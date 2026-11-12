using System;
using System.Collections.Generic;

using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.Serialization;

namespace Greyhound.Utilities
{
    // Identity of a part of an imported model: the model asset (`root`)
    // and the path of sibling indices from that model's root down to the
    // part GameObject (root child n, its child m, ...). An imported STEP
    // hierarchy rebuilds deterministically per file, so the index path is
    // rename-robust where a name key is not, and it distinguishes
    // duplicate-named parts for free. `targetName` carries the
    // creation-time name as a display reference.
    //
    // States of `indices`:
    //   []      no target set yet.
    //   [-1]    the target is the model root itself (SELF).
    //   [2, 0]  root's child 2, then its child 0. Indices are >= 0.
    // SELF is only ever the whole path, never one step of a longer one.
    //
    // `root` is the model asset, never a scene object: the holder may be a
    // ScriptableObject, which cannot reference scene objects. The scene
    // copy is found when needed (FindInstance) and the part under it
    // (Resolve).
    //
    // Serialized form: the field names `root`, `indices` and `targetName`.
    // `targetName` was `name` in Greyhound.Step.StepSiblingPath; the
    // FormerlySerializedAs keeps that stored data loading. Unity
    // serializes a [Serializable] class by field names, not its type name,
    // so data stored under the old type names loads unchanged.
    //
    // Deferred design (out of scope for now): on reimport, a conflict
    // resolution flow compares the resolved part's name against the
    // stored one — a rename is accepted with one click, a structural
    // change surfaces for re-picking — instead of silently rebinding
    // onto whatever now sits at the indexed position.
    [Serializable]
    public sealed class SiblingPath
    {
        // `indices` = [SELF] targets the root itself.
        public const int SELF = -1;

        public GameObject root;
        public int[] indices = Array.Empty<int>();
        public string targetName;

        // The target in the loaded scenes: the instance of `root`
        // (FindInstance), then the walk down `indices` (ResolveIn). Null
        // when there is no root, no target, no instance, or the path no
        // longer fits.
        public Transform Resolve(bool matchByName)
        {
            return root == null ? null : ResolveIn(FindInstance(root, matchByName));
        }

        // The target below a given instance of `root`: the instance itself
        // for [SELF], else the walk down the child indices. Example: [2, 0]
        // is instance.GetChild(2).GetChild(0). Null for an empty path, no
        // instance, or an index past the children.
        public Transform ResolveIn(Transform instance)
        {
            if (instance == null || indices == null || indices.Length == 0)
            {
                return null;
            }
            if (IsSelf(indices))
            {
                return instance;
            }
            Transform node = instance;
            foreach (int index in indices)
            {
                if (index < 0 || index >= node.childCount)
                {
                    return null;
                }
                node = node.GetChild(index);
            }
            return node;
        }

        public static bool IsSelf(int[] indices)
        {
            return indices != null && indices.Length == 1 && indices[0] == SELF;
        }

        // The scene instance of a model asset, searched in hierarchy order
        // across the loaded scenes:
        //   - in the editor, the first prefab instance root whose original
        //     source is that asset;
        //   - with matchByName, failing that (and always in a player, where
        //     prefab links do not exist), an active root GameObject named
        //     like the asset. Unity gives no order here, so a scene with
        //     several same-named roots gets an arbitrary one of them; keep
        //     one instance of a driven model per scene.
        // The editor tools pass matchByName false: they follow prefab links
        // only. The runtime passes true.
        //
        // The name match searches existing objects (FindObjectsByType), not
        // the loaded scenes' roots: in a built player, a walk of the loaded
        // scenes' roots found no instance at a StateServer's Awake although
        // the model sat at the scene root (observed in the WIP build; the
        // scene presumably still counts as loading then).
        public static Transform FindInstance(GameObject rootAsset, bool matchByName)
        {
            if (rootAsset == null)
            {
                return null;
            }
#if UNITY_EDITOR
            for (int i = 0; i < SceneManager.sceneCount; i++)
            {
                Scene scene = SceneManager.GetSceneAt(i);
                if (!scene.isLoaded)
                {
                    continue;
                }
                foreach (GameObject sceneRoot in scene.GetRootGameObjects())
                {
                    Transform found = FindPrefabInstance(sceneRoot.transform, rootAsset);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
#endif
            if (!matchByName)
            {
                return null;
            }
            foreach (Transform candidate in UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None))
            {
                if (candidate.parent == null && candidate.name == rootAsset.name)
                {
                    return candidate;
                }
            }
            return null;
        }

        // The indices of `node` below `instanceRoot`: [SELF] for the root
        // itself, else the child indices. Example: root -> A (index 1) ->
        // B (index 0) gives [1, 0]. `node` must be instanceRoot or below it.
        public static int[] IndicesOf(Transform node, Transform instanceRoot)
        {
            if (node == instanceRoot)
            {
                return new[] { SELF };
            }
            var reversed = new List<int>();
            for (Transform step = node; step != null && step != instanceRoot; step = step.parent)
            {
                reversed.Add(step.GetSiblingIndex());
            }
            reversed.Reverse();
            return reversed.ToArray();
        }

#if UNITY_EDITOR
        // Editor only (prefab links do not exist in a player): the nearest
        // ancestor-or-self of a scene GameObject that is the instance root
        // of an asset (its original source is a main asset), and that
        // asset. False when there is none. Example: a part of an imported
        // arm.step instance walks up to the instance root, whose source is
        // arm.step's root GameObject.
        public static bool TryFindInstanceRoot(Transform node, out Transform instanceRoot, out GameObject rootAsset)
        {
            for (Transform candidate = node; candidate != null; candidate = candidate.parent)
            {
                GameObject source = UnityEditor.PrefabUtility.GetCorrespondingObjectFromOriginalSource(candidate.gameObject);
                if (source != null && UnityEditor.AssetDatabase.IsMainAsset(source))
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

        private static Transform FindPrefabInstance(Transform node, GameObject rootAsset)
        {
            if (UnityEditor.PrefabUtility.IsAnyPrefabInstanceRoot(node.gameObject) &&
                UnityEditor.PrefabUtility.GetCorrespondingObjectFromOriginalSource(node.gameObject) == rootAsset)
            {
                return node;
            }
            foreach (Transform child in node)
            {
                Transform found = FindPrefabInstance(child, rootAsset);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }
#endif
    }
}
