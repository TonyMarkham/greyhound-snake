using System.Collections.Generic;

using UnityEditor;
using UnityEngine;

namespace Greyhound.Step
{
    // Hierarchy right-click / GameObject menu entries for joint and
    // actuator authoring. The parent of a joint is never picked: it is
    // the part's hierarchy parent. The root itself is not jointable —
    // its mobility against the world is the rootMobility setting.
    // Joints key on STEP part names, so validation and actuator
    // targeting resolve through the container asset; nothing is
    // attached to the part GameObjects.
    internal static class StepJointMenu
    {
        private const int MenuPriority = 10;

        [MenuItem("GameObject/Add/Joint/Slide", false, MenuPriority)]
        private static void AddSlideJoint(MenuCommand command)
        {
            DeferJoint(command, StepJointType.Slide);
        }

        [MenuItem("GameObject/Add/Joint/Slide", true)]
        private static bool ValidateSlideJoint()
        {
            return ValidateJoint();
        }

        [MenuItem("GameObject/Add/Joint/Hinge", false, MenuPriority)]
        private static void AddHingeJoint(MenuCommand command)
        {
            DeferJoint(command, StepJointType.Hinge);
        }

        [MenuItem("GameObject/Add/Joint/Hinge", true)]
        private static bool ValidateHingeJoint()
        {
            return ValidateJoint();
        }

        [MenuItem("GameObject/Add/Actuator", false, MenuPriority)]
        private static void AddActuator(MenuCommand command)
        {
            GameObject part = SelectedPart(command);
            if (part == null)
            {
                return;
            }
            Defer(() => AddActuatorInternal(part));
        }

        [MenuItem("GameObject/Add/Actuator", true)]
        private static bool ValidateActuator()
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
            // Exactly one: zero means nothing to actuate, more than one
            // means the position is already annotated several times.
            return StepJointSetFactory.FindJoints(set, part, instanceRoot).Count == 1;
        }

        private static void DeferJoint(MenuCommand command, StepJointType type)
        {
            GameObject part = SelectedPart(command);
            if (part == null)
            {
                return;
            }
            Defer(() => AddJointInternal(part, type));
        }

        private static void AddJointInternal(GameObject part, StepJointType type)
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

            StepJointSetFactory.WarnDuplicatePartNames(set, instanceRoot, part);
            StepJoint joint = StepJointSetFactory.CreateJointAsset(set, instanceRoot, part, type);
            Select(joint);
        }

        private static void AddActuatorInternal(GameObject part)
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

            List<StepJoint> joints = StepJointSetFactory.FindJoints(set, part, instanceRoot);
            if (joints.Count == 0)
            {
                Debug.LogError($"{part.name}: no joint to actuate");
                return;
            }
            if (joints.Count > 1)
            {
                Debug.LogError(
                    $"{part.name}: {joints.Count} joints annotate this position; " +
                    "the actuator target is ambiguous");
                return;
            }

            StepActuator actuator = StepJointSetFactory.CreateActuatorAsset(set, joints[0], part);
            Select(actuator);
        }

        // Unity 6.7's rewritten Hierarchy frames-and-renames the "new"
        // object after any GameObject menu invocation, dereferencing it
        // as a GameObject. Creating assets or selecting one inside the
        // menu callback trips that (a class-114 asset at fileID 11400000
        // cannot cast to GameObject), so the work runs one editor tick
        // later, outside the menu context.
        private static void Defer(System.Action action)
        {
            EditorApplication.delayCall += () => action();
        }

        private static bool ValidateJoint()
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
            if (part == instanceRoot)
            {
                return false;
            }
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return true;
            }
            return StepJointSetFactory.FindJoints(set, part, instanceRoot).Count == 0;
        }

        private static GameObject SelectedPart(MenuCommand command)
        {
            return command.context as GameObject ?? Selection.activeGameObject;
        }

        private static void Select(Object asset)
        {
            Selection.activeObject = asset;
            EditorGUIUtility.PingObject(asset);
        }
    }
}
