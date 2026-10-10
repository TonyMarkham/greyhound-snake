using System.Collections.Generic;
using System.IO;

using UnityEditor;

using UnityEngine;

namespace Greyhound.Step
{
    // Reimport survival: a reimport rebuilds the imported hierarchy, so
    // after each .stp reimport the annotation family's sibling paths are
    // validated against the fresh structure. Every joint's path must
    // resolve in the rebuilt instance; the joints that no longer place
    // surface as Console errors with counts — never silently dropped.
    // Nothing is restored or rewritten: paths that still resolve need no
    // repair, and the deferred conflict flow (resolved name vs stored
    // name) is a later bite. The check runs one editor tick after the
    // import pass so the scene instances have synced to the rewritten
    // prefab asset.
    internal sealed class StepReimportValidator : AssetPostprocessor
    {
        private static void OnPostprocessAllAssets(
            string[] importedAssets,
            string[] deletedAssets,
            string[] movedAssets,
            string[] movedFromAssetPaths,
            bool didDomainReload)
        {
            foreach (string assetPath in importedAssets)
            {
                if (!StepJointSetFactory.HasStepExtension(assetPath))
                {
                    continue;
                }
                string stepPath = assetPath;
                EditorApplication.delayCall += () => Validate(stepPath);
            }
        }

        private static void Validate(string stepPath)
        {
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return;
            }
            // No instance of the .stp is loaded: there is nothing to
            // validate against, and no verdict to report.
            if (StepJointSetFactory.ResolveInstanceRoot(stepPath) == null)
            {
                return;
            }

            // Container hygiene: slots whose asset was deleted outside
            // the pruner's reach go now.
            bool pruned = false;
            for (int i = set.joints.Count - 1; i >= 0; i--)
            {
                if (set.joints[i] == null)
                {
                    set.joints.RemoveAt(i);
                    pruned = true;
                }
            }
            for (int i = set.actuators.Count - 1; i >= 0; i--)
            {
                if (set.actuators[i] == null)
                {
                    set.actuators.RemoveAt(i);
                    pruned = true;
                }
            }
            for (int i = set.geoms.Count - 1; i >= 0; i--)
            {
                if (set.geoms[i] == null)
                {
                    set.geoms.RemoveAt(i);
                    pruned = true;
                }
            }
            if (pruned)
            {
                EditorUtility.SetDirty(set);
            }

            var unmatched = new List<string>();
            int total = 0;
            foreach (StepJoint joint in set.joints)
            {
                if (joint == null)
                {
                    continue;
                }
                total++;
                if (StepJointSetFactory.FindPart(set, joint.body) == null)
                {
                    unmatched.Add(joint.name);
                }
            }
            if (unmatched.Count > 0)
            {
                Debug.LogError(
                    $"{stepPath}: {unmatched.Count} of {total} joints could not be placed " +
                    $"after reimport: {string.Join(", ", unmatched)}");
            }

            var unmatchedGeoms = new List<string>();
            int geomTotal = 0;
            foreach (StepGeom geom in set.geoms)
            {
                if (geom == null)
                {
                    continue;
                }
                geomTotal++;
                if (StepJointSetFactory.FindPart(set, geom.body) == null)
                {
                    unmatchedGeoms.Add(geom.name);
                }
            }
            if (unmatchedGeoms.Count > 0)
            {
                Debug.LogError(
                    $"{stepPath}: {unmatchedGeoms.Count} of {geomTotal} geoms could not be placed " +
                    $"after reimport: {string.Join(", ", unmatchedGeoms)}");
            }
        }
    }
}
