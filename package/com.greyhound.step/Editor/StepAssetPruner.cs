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

            var geom = AssetDatabase.LoadAssetAtPath<StepGeom>(assetPath);
            if (geom != null && geom.root != null)
            {
                Undo.RecordObject(geom.root, "Prune Deleted Geom");
                geom.root.geoms.Remove(geom);
                EditorUtility.SetDirty(geom.root);
            }

            var actuator = AssetDatabase.LoadAssetAtPath<StepActuator>(assetPath);
            if (actuator != null && actuator.root != null)
            {
                Undo.RecordObject(actuator.root, "Prune Deleted Actuator");
                actuator.root.actuators.Remove(actuator);
                EditorUtility.SetDirty(actuator.root);
            }

            var site = AssetDatabase.LoadAssetAtPath<StepSite>(assetPath);
            if (site != null && site.root != null)
            {
                Undo.RecordObject(site.root, "Prune Deleted Site");
                site.root.sites.Remove(site);
                EditorUtility.SetDirty(site.root);
            }

            var sensor = AssetDatabase.LoadAssetAtPath<StepSensor>(assetPath);
            if (sensor != null && sensor.root != null)
            {
                Undo.RecordObject(sensor.root, "Prune Deleted Sensor");
                sensor.root.sensors.Remove(sensor);
                EditorUtility.SetDirty(sensor.root);
            }

            return AssetDeleteResult.DidNotDelete;
        }
    }
}
