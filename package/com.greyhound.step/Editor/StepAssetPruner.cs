using UnityEditor;

using UnityEngine;

namespace Greyhound.Step
{
    // Lifecycle: when a joint or actuator asset is deleted, it is
    // removed from its set's list at deletion time instead of leaving a
    // null slot behind. Deletion itself stays with Unity
    // (DidNotDelete); the container is only kept honest.
    internal sealed class StepAssetPruner : AssetModificationProcessor
    {
        private static AssetDeleteResult OnWillDeleteAsset(string assetPath, RemoveAssetOptions options)
        {
            var joint = AssetDatabase.LoadAssetAtPath<StepJoint>(assetPath);
            if (joint != null && joint.root != null)
            {
                Undo.RecordObject(joint.root, "Prune Deleted Joint");
                joint.root.joints.Remove(joint);
                EditorUtility.SetDirty(joint.root);
            }

            var actuator = AssetDatabase.LoadAssetAtPath<StepActuator>(assetPath);
            if (actuator != null && actuator.root != null)
            {
                Undo.RecordObject(actuator.root, "Prune Deleted Actuator");
                actuator.root.actuators.Remove(actuator);
                EditorUtility.SetDirty(actuator.root);
            }

            return AssetDeleteResult.DidNotDelete;
        }
    }
}
