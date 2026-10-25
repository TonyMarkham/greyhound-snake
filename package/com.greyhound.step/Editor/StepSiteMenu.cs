using System.Collections.Generic;

using UnityEditor;

using UnityEngine;

namespace Greyhound.Step
{
    // Hierarchy right-click / GameObject menu entries for site and
    // sensor authoring. A site attaches to any STEP part, the assembly
    // root included (a welded root is fixed to the world), one site per
    // part as with joints and geoms. A sensor is added on a part that
    // carries a site and measures that site; its optional reference site
    // is picked in the sensor's inspector. Both key on the sibling-index
    // path like every element of the annotation family; nothing is
    // attached to the part GameObjects.
    internal static class StepSiteMenu
    {
        private const int MenuPriority = 10;

        [MenuItem("GameObject/Add/Site", false, MenuPriority)]
        private static void AddSite(MenuCommand command)
        {
            GameObject part = command.context as GameObject ?? Selection.activeGameObject;
            if (part == null)
            {
                return;
            }
            // Unity 6.7's rewritten Hierarchy frames-and-renames the
            // "new" object after any GameObject menu invocation, so the
            // work runs one editor tick later, outside the menu context
            // (the joint menu's Defer pattern).
            EditorApplication.delayCall += () => AddSiteInternal(part);
        }

        [MenuItem("GameObject/Add/Site", true)]
        private static bool ValidateSite()
        {
            GameObject part = Selection.activeGameObject;
            if (part == null)
            {
                return false;
            }
            if (!StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath))
            {
                return false;
            }
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return true;
            }
            return StepJointSetFactory.FindSites(set, part, instanceRoot).Count == 0;
        }

        [MenuItem("GameObject/Add/Sensor", false, MenuPriority)]
        private static void AddSensor(MenuCommand command)
        {
            GameObject part = command.context as GameObject ?? Selection.activeGameObject;
            if (part == null)
            {
                return;
            }
            EditorApplication.delayCall += () => AddSensorInternal(part);
        }

        [MenuItem("GameObject/Add/Sensor", true)]
        private static bool ValidateSensor()
        {
            GameObject part = Selection.activeGameObject;
            if (part == null)
            {
                return false;
            }
            if (!StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath))
            {
                return false;
            }
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return false;
            }
            // Exactly one: zero means nothing to measure, more than one
            // means the position is already annotated several times.
            return StepJointSetFactory.FindSites(set, part, instanceRoot).Count == 1;
        }

        private static void AddSiteInternal(GameObject part)
        {
            if (!StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath))
            {
                Debug.LogError($"{part.name}: not a STEP-imported part");
                return;
            }
            StepJointSet set = StepJointSetFactory.FindOrCreateSet(stepPath);
            if (set == null)
            {
                return;
            }

            StepSite site = StepJointSetFactory.CreateSiteAsset(set, instanceRoot, part);
            Select(site);
        }

        private static void AddSensorInternal(GameObject part)
        {
            if (!StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath))
            {
                return;
            }
            StepJointSet set = StepJointSetFactory.FindOrCreateSet(stepPath);
            if (set == null)
            {
                return;
            }

            List<StepSite> sites = StepJointSetFactory.FindSites(set, part, instanceRoot);
            if (sites.Count == 0)
            {
                Debug.LogError($"{part.name}: no site to measure");
                return;
            }
            if (sites.Count > 1)
            {
                Debug.LogError(
                    $"{part.name}: {sites.Count} sites annotate this position; " +
                    "the sensor target is ambiguous");
                return;
            }

            StepSensor sensor = StepJointSetFactory.CreateSensorAsset(set, sites[0], part);
            Select(sensor);
        }

        private static void Select(Object asset)
        {
            Selection.activeObject = asset;
            EditorGUIUtility.PingObject(asset);
        }
    }
}
