using UnityEditor;

using UnityEngine;

namespace Greyhound.Step
{
    // Hierarchy right-click / GameObject menu entry for geom authoring.
    // The geom attaches to a meshed part — the MeshFilter is what makes
    // the part exportable as a mesh geom — and keys on the sibling-index
    // path like every element of the annotation family; nothing is
    // attached to the part GameObjects.
    internal static class StepGeomMenu
    {
        private const int MenuPriority = 10;

        [MenuItem("GameObject/Add/Geom", false, MenuPriority)]
        private static void AddGeom(MenuCommand command)
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
            EditorApplication.delayCall += () => AddGeomInternal(part);
        }

        [MenuItem("GameObject/Add/Geom", true)]
        private static bool ValidateGeom()
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
            if (part.GetComponent<MeshFilter>() == null)
            {
                return false;
            }
            StepJointSet set = StepJointSetFactory.FindSet(stepPath);
            if (set == null)
            {
                return true;
            }
            return StepJointSetFactory.FindGeoms(set, part, instanceRoot).Count == 0;
        }

        private static void AddGeomInternal(GameObject part)
        {
            if (!StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath))
            {
                Debug.LogError($"{part.name}: not a STEP-imported part");
                return;
            }
            if (part.GetComponent<MeshFilter>() == null)
            {
                Debug.LogError($"{part.name}: geoms attach to meshed parts");
                return;
            }
            StepJointSet set = StepJointSetFactory.FindOrCreateSet(stepPath);
            if (set == null)
            {
                return;
            }

            StepJointSetFactory.WarnDuplicatePartNames(set, instanceRoot, part);
            StepGeom geom = StepJointSetFactory.CreateGeomAsset(set, instanceRoot, part);
            Selection.activeObject = geom;
            EditorGUIUtility.PingObject(geom);
        }
    }
}
