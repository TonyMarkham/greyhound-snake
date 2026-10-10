using System.Collections.Generic;

using UnityEditor;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // Renders a StepSiblingPath as an immutable hierarchy accordion: the
    // live chain from the assembly root to the annotated part (current
    // names at the stored positions), with the creation-time name
    // alongside. The identity stays the stored index path + name; this
    // is presentation only — nothing here is editable. A drop zone
    // below rebinds the path to a dragged part of the same STEP
    // instance: the re-pick affordance, writing through the serialized
    // property so undo applies.
    [CustomPropertyDrawer(typeof(StepSiblingPath))]
    internal sealed class StepSiblingPathDrawer : PropertyDrawer
    {
        public override VisualElement CreatePropertyGUI(SerializedProperty property)
        {
            var container = new VisualElement();
            container.AddToClassList("sibling-path");

            var joint = property.serializedObject.targetObject as StepJoint;
            string stepPath = joint != null && joint.root != null ? joint.root.stepAssetPath : null;

            Rebuild(container, property, stepPath);
            return container;
        }

        private static void Rebuild(VisualElement container, SerializedProperty property, string stepPath)
        {
            container.Clear();
            StepSiblingPath stored = ReadPath(property);

            List<Transform> chain = string.IsNullOrEmpty(stepPath)
                ? new List<Transform>()
                : StepJointSetFactory.ResolveChain(stepPath, stored);
            Transform target = chain.Count > 0 ? chain[chain.Count - 1] : null;
            string liveName = target != null ? target.name : "(unresolved)";

            var foldout = new Foldout { text = liveName, value = false };
            foldout.AddToClassList("sibling-path-foldout");
            container.Add(foldout);

            var rows = new VisualElement();
            rows.AddToClassList("sibling-path-chain");
            foldout.Add(rows);

            for (int i = 0; i < chain.Count; i++)
            {
                Transform node = chain[i];
                Label row;
                if (node == null)
                {
                    row = new Label("(missing)");
                }
                else if (i == 0)
                {
                    row = new Label(node.name);
                }
                else
                {
                    row = new Label($"{node.name}  [{stored.indices[i - 1]}]");
                }
                row.AddToClassList(i == chain.Count - 1
                    ? "sibling-path-chain__row--target"
                    : "sibling-path-chain__row");
                rows.Add(row);
            }

            var createdRow = new Label($"created for: {(stored != null ? stored.name : "(none)")}");
            createdRow.AddToClassList("sibling-path-chain__row");
            rows.Add(createdRow);

            var dropZone = new VisualElement();
            dropZone.AddToClassList("sibling-path-dropzone");
            dropZone.Add(new Label("drop a part to rebind"));
            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = TryGetValidDrop(stepPath, out _, out _)
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                if (!TryGetValidDrop(stepPath, out GameObject dropped, out GameObject instanceRoot))
                {
                    return;
                }
                StepSiblingPath newPath = StepJointSetFactory.PathOf(dropped, instanceRoot);
                WritePath(property, newPath);
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
                Rebuild(container, property, stepPath);
            });
            container.Add(dropZone);
        }

        // A drop is valid when the dragged object is a part of the same
        // STEP instance the joint annotates; foreign drops are rejected.
        private static bool TryGetValidDrop(
            string stepPath, out GameObject dropped, out GameObject instanceRoot)
        {
            dropped = null;
            instanceRoot = null;
            foreach (Object reference in DragAndDrop.objectReferences)
            {
                if (reference is GameObject candidate &&
                    StepJointSetFactory.TryResolveStepSource(candidate, out GameObject root, out string source) &&
                    source == stepPath)
                {
                    dropped = candidate;
                    instanceRoot = root;
                    return true;
                }
            }
            return false;
        }

        private static StepSiblingPath ReadPath(SerializedProperty property)
        {
            SerializedProperty indices = property.FindPropertyRelative("indices");
            var path = new List<int>(indices.arraySize);
            for (int i = 0; i < indices.arraySize; i++)
            {
                path.Add(indices.GetArrayElementAtIndex(i).intValue);
            }
            return new StepSiblingPath
            {
                indices = path.ToArray(),
                name = property.FindPropertyRelative("name").stringValue,
            };
        }

        private static void WritePath(SerializedProperty property, StepSiblingPath path)
        {
            SerializedProperty indices = property.FindPropertyRelative("indices");
            indices.arraySize = path.indices.Length;
            for (int i = 0; i < path.indices.Length; i++)
            {
                indices.GetArrayElementAtIndex(i).intValue = path.indices[i];
            }
            property.FindPropertyRelative("name").stringValue = path.name;
            property.serializedObject.ApplyModifiedProperties();
        }
    }
}
