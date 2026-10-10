using System.Collections.Generic;
using System.IO;
using System.Linq;

using UnityEditor;
using UnityEditor.SceneManagement;

using UnityEngine;
using UnityEngine.SceneManagement;

namespace Greyhound.Step
{
    // Locate-or-create logic for the joint set folder beside a STEP file
    // and for the joint/actuator assets inside it. The container asset
    // lives at a deterministic path ("<stem>.joints/<stem>.joints.asset"
    // next to the .stp), so lookup is by path; joints and actuators get
    // unique file names to survive repeated creation.
    internal static class StepJointSetFactory
    {
        public static bool TryResolveStepSource(
            GameObject part, out GameObject instanceRoot, out string stepPath)
        {
            instanceRoot = null;
            stepPath = null;

            // Prefab mode edits the asset itself, not an instance; the
            // stage's asset path is the .stp directly.
            PrefabStage stage = PrefabStageUtility.GetCurrentPrefabStage();
            if (stage != null && stage.prefabContentsRoot != null)
            {
                if (!IsSelfOrDescendant(stage.prefabContentsRoot.transform, part.transform))
                {
                    return false;
                }
                instanceRoot = stage.prefabContentsRoot;
                stepPath = stage.assetPath;
                return HasStepExtension(stepPath);
            }

            // Imported parts are plain children of one prefab instance, so
            // the nearest instance root of any part is the assembly root.
            GameObject sourceRoot = PrefabUtility.GetNearestPrefabInstanceRoot(part);
            if (sourceRoot == null)
            {
                return false;
            }
            string path = PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(sourceRoot);
            if (!HasStepExtension(path))
            {
                return false;
            }
            instanceRoot = sourceRoot;
            stepPath = path;
            return true;
        }

        public static StepJointSet FindSet(string stepPath)
        {
            return AssetDatabase.LoadAssetAtPath<StepJointSet>(ContainerPath(stepPath));
        }

        public static StepJointSet FindOrCreateSet(string stepPath)
        {
            string containerPath = ContainerPath(stepPath);
            StepJointSet set = AssetDatabase.LoadAssetAtPath<StepJointSet>(containerPath);
            if (set != null)
            {
                // The path is deterministic, so a container found here
                // annotates this .stp; reconcile a stale key (the .stp
                // moved together with its folder).
                if (set.stepAssetPath != stepPath)
                {
                    set.stepAssetPath = stepPath;
                    EditorUtility.SetDirty(set);
                }
                return set;
            }

            string directory = Path.GetDirectoryName(stepPath);
            if (directory == null || (directory != "Assets" && !directory.StartsWith("Assets/")))
            {
                Debug.LogError(
                    $"{stepPath}: joint sets can only be created beside assets under Assets/");
                return null;
            }

            string folder = Path.GetDirectoryName(containerPath);
            if (!AssetDatabase.IsValidFolder(folder))
            {
                AssetDatabase.CreateFolder(directory, Path.GetFileName(folder));
            }

            set = ScriptableObject.CreateInstance<StepJointSet>();
            set.name = Path.GetFileNameWithoutExtension(containerPath);
            set.stepAssetPath = stepPath;
            AssetDatabase.CreateAsset(set, AssetDatabase.GenerateUniqueAssetPath(containerPath));
            Undo.RegisterCreatedObjectUndo(set, "Create Step Joint Set");
            return set;
        }

        public static StepJoint CreateJointAsset(
            StepJointSet set, GameObject instanceRoot, GameObject part, StepJointType type)
        {
            var joint = ScriptableObject.CreateInstance<StepJoint>();
            joint.name = part.name;
            joint.root = set;
            joint.body = PathOf(part, instanceRoot);
            joint.mj.type = type;

            string folder = SetFolder(set);
            string path = AssetDatabase.GenerateUniqueAssetPath(
                $"{folder}/{Sanitize(part.name)}.joint.asset");
            AssetDatabase.CreateAsset(joint, path);
            Undo.RegisterCreatedObjectUndo(joint, "Add Step Joint");

            Undo.RecordObject(set, "Add Step Joint");
            set.joints.Add(joint);
            EditorUtility.SetDirty(set);
            return joint;
        }

        // The joint's identity is the sibling-index path, so a part's
        // joint is found by comparing the part's live path against the
        // stored paths. More than one match is ambiguous; callers decide
        // how to report it.
        public static List<StepJoint> FindJoints(StepJointSet set, GameObject part, GameObject instanceRoot)
        {
            int[] path = PathIndices(part, instanceRoot);
            return set.joints
                .Where(joint => joint != null && joint.body != null && SamePath(joint.body.indices, path))
                .ToList();
        }

        // Scene-side resolution: the part GameObject a joint annotates.
        // The annotation family is self-sufficient — the part is found by
        // walking the sibling-index path through the set's stepAssetPath
        // instance, in the open prefab stage or in the loaded scenes.
        public static GameObject FindPart(StepJoint joint)
        {
            return joint != null && joint.body != null
                ? FindPart(joint.root, joint.body)
                : null;
        }

        public static GameObject FindPart(StepJointSet set, StepSiblingPath path)
        {
            if (set == null || path == null || string.IsNullOrEmpty(set.stepAssetPath))
            {
                return null;
            }

            Transform root = ResolveInstanceRoot(set.stepAssetPath);
            if (root == null)
            {
                return null;
            }

            Transform node = root;
            foreach (int index in path.indices)
            {
                if (node == null || index < 0 || index >= node.childCount)
                {
                    return null;
                }
                node = node.GetChild(index);
            }
            return node.gameObject;
        }

        // The creation-time identity of a part: its sibling-index path
        // from the assembly root plus its name as the display reference.
        public static StepSiblingPath PathOf(GameObject part, GameObject instanceRoot)
        {
            return new StepSiblingPath
            {
                indices = PathIndices(part, instanceRoot),
                name = part.name,
            };
        }

        // The chain of nodes a path walks through, assembly root first
        // and the annotated part last; trailing nulls where the path no
        // longer resolves (a shifted index path). Presentation- and
        // conflict-facing; keying and matching use the raw paths.
        public static List<Transform> ResolveChain(string stepAssetPath, StepSiblingPath path)
        {
            var chain = new List<Transform>();
            Transform root = ResolveInstanceRoot(stepAssetPath);
            if (root == null)
            {
                return chain;
            }
            chain.Add(root);

            Transform node = root;
            foreach (int index in path.indices)
            {
                node = node != null && index >= 0 && index < node.childCount
                    ? node.GetChild(index)
                    : null;
                chain.Add(node);
            }
            return chain;
        }

        private static Transform ResolveInstanceRoot(string stepAssetPath)
        {
            PrefabStage stage = PrefabStageUtility.GetCurrentPrefabStage();
            if (stage != null && stage.assetPath == stepAssetPath && stage.prefabContentsRoot != null)
            {
                return stage.prefabContentsRoot.transform;
            }

            for (int i = 0; i < SceneManager.sceneCount; i++)
            {
                foreach (GameObject sceneRoot in SceneManager.GetSceneAt(i).GetRootGameObjects())
                {
                    Transform found = FindInstanceRoot(sceneRoot.transform, stepAssetPath);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
            return null;
        }

        private static Transform FindInstanceRoot(Transform node, string stepAssetPath)
        {
            GameObject instanceRoot = PrefabUtility.GetNearestPrefabInstanceRoot(node.gameObject);
            if (instanceRoot != null &&
                PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(instanceRoot) == stepAssetPath)
            {
                return instanceRoot.transform;
            }
            foreach (Transform child in node)
            {
                Transform found = FindInstanceRoot(child, stepAssetPath);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }

        private static int[] PathIndices(GameObject part, GameObject instanceRoot)
        {
            var reversed = new List<int>();
            Transform node = part.transform;
            while (node != null && node.gameObject != instanceRoot)
            {
                reversed.Add(node.GetSiblingIndex());
                node = node.parent;
            }
            reversed.Reverse();
            return reversed.ToArray();
        }

        private static bool SamePath(int[] stored, int[] live)
        {
            return stored != null && live != null &&
                stored.Length == live.Length &&
                stored.Zip(live, (a, b) => a == b).All(equal => equal);
        }

        public static StepActuator CreateActuatorAsset(StepJointSet set, StepJoint joint, GameObject part)
        {
            var actuator = ScriptableObject.CreateInstance<StepActuator>();
            actuator.name = $"{part.name}_pos";
            actuator.root = set;
            actuator.target = joint;
            actuator.mj.type = StepActuatorType.Position;

            string folder = SetFolder(set);
            string path = AssetDatabase.GenerateUniqueAssetPath(
                $"{folder}/{Sanitize(part.name)}.actuator.asset");
            AssetDatabase.CreateAsset(actuator, path);
            Undo.RegisterCreatedObjectUndo(actuator, "Add Step Actuator");

            Undo.RecordObject(set, "Add Step Actuator");
            set.actuators.Add(actuator);
            EditorUtility.SetDirty(set);
            return actuator;
        }

        // Part names feed the MJCF body names at export, where duplicates
        // will collide — an M3 concern flagged early. Keying itself is
        // the sibling-index path, so a repeated name is never ambiguous
        // for the annotation.
        public static void WarnDuplicatePartNames(StepJointSet set, GameObject instanceRoot, GameObject part)
        {
            Transform[] transforms = instanceRoot.GetComponentsInChildren<Transform>(true);
            int sameName = transforms.Count(t => t != part.transform && t.name == part.name);
            if (sameName > 0)
            {
                Debug.LogWarning(
                    $"{set.stepAssetPath}: part name '{part.name}' occurs {sameName + 1} times " +
                    "in the assembly; duplicate body names collide in the exported MJCF (M3).");
            }
        }

        private static bool HasStepExtension(string path)
        {
            string extension = Path.GetExtension(path).ToLowerInvariant();
            return extension == ".stp" || extension == ".step";
        }

        private static string ContainerPath(string stepPath)
        {
            string stem = Path.GetFileNameWithoutExtension(stepPath);
            string directory = Path.GetDirectoryName(stepPath);
            return $"{directory}/{stem}.joints/{stem}.joints.asset";
        }

        private static string SetFolder(StepJointSet set)
        {
            string containerPath = AssetDatabase.GetAssetPath(set);
            return Path.GetDirectoryName(containerPath);
        }

        private static bool IsSelfOrDescendant(Transform root, Transform node)
        {
            while (node != null)
            {
                if (node == root)
                {
                    return true;
                }
                node = node.parent;
            }
            return false;
        }

        private static string Sanitize(string name)
        {
            return string.Concat(
                name.Select(character =>
                    Path.GetInvalidFileNameChars().Contains(character) ? '_' : character));
        }
    }
}
