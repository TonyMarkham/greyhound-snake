using System.Collections.Generic;

using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The sensor asset's inspector: a bound UXML form over the target
    // and reference sites. The UI assets are serialized references
    // assigned on this script asset (the joint-family editor pattern).
    // Each site field has a drop zone below it: drop a part of the
    // set's STEP instance that carries one site, and that site fills the
    // field, writing through the serialized property so undo applies.
    // The form warns on what the exporter refuses: no target, or a
    // target or reference that is not a site of this sensor's set.
    [CustomEditor(typeof(StepSensor))]
    internal sealed class StepSensorEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepSensorEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepSensorEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            RegisterSiteDrop(root.Q<VisualElement>("target-drop"), "target");
            RegisterSiteDrop(root.Q<VisualElement>("reference-drop"), "reference");

            Label targetWarning = root.Q<Label>("target-warning");
            Label setWarning = root.Q<Label>("set-warning");
            UpdateValidation(targetWarning, setWarning);
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                UpdateValidation(targetWarning, setWarning));
            return root;
        }

        private void RegisterSiteDrop(VisualElement dropZone, string propertyName)
        {
            if (dropZone == null)
            {
                return;
            }
            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = TryGetDroppedSite(out _)
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                if (!TryGetDroppedSite(out StepSite site))
                {
                    return;
                }
                serializedObject.Update();
                serializedObject.FindProperty(propertyName).objectReferenceValue = site;
                serializedObject.ApplyModifiedProperties();
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
            });
        }

        // A drop is valid when the dragged object is a part of the STEP
        // instance the sensor's set annotates and carries exactly one
        // site; a sensor with no Root set rejects every drop.
        private bool TryGetDroppedSite(out StepSite site)
        {
            site = null;
            var sensor = (StepSensor)target;
            if (sensor == null || sensor.root == null)
            {
                return false;
            }
            foreach (Object reference in DragAndDrop.objectReferences)
            {
                if (reference is GameObject part &&
                    StepJointSetFactory.TryResolveStepSource(part, out GameObject instanceRoot, out string stepPath) &&
                    stepPath == sensor.root.stepAssetPath)
                {
                    List<StepSite> sites = StepJointSetFactory.FindSites(sensor.root, part, instanceRoot);
                    if (sites.Count == 1)
                    {
                        site = sites[0];
                        return true;
                    }
                }
            }
            return false;
        }

        private void UpdateValidation(Label targetWarning, Label setWarning)
        {
            var sensor = (StepSensor)target;
            if (sensor == null)
            {
                return;
            }
            targetWarning.style.display =
                sensor.target == null ? DisplayStyle.Flex : DisplayStyle.None;
            bool outside = sensor.root != null
                && ((sensor.target != null && !sensor.root.sites.Contains(sensor.target))
                    || (sensor.reference != null && !sensor.root.sites.Contains(sensor.reference)));
            setWarning.style.display = outside ? DisplayStyle.Flex : DisplayStyle.None;
        }
    }
}
