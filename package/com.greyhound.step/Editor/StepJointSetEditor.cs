using System.Collections.Generic;
using System.Linq;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // The container's inspector doubles as the set overview: the two
    // ListViews of joints and actuators ping-select entries — the joint
    // selection activates its inspector and scene tools — instead of a
    // dropped authoring window. ListView has lived in
    // UnityEngine.UIElements since Unity 6, so the UXML spells it
    // ui:ListView (the legacy UnityEditor.UIElements namespace has no
    // UxmlElement descriptor and fails to instantiate).
    [CustomEditor(typeof(StepJointSet))]
    internal sealed class StepJointSetEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;
        
        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepJointSetEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();
            
            if(m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(StepJointSetEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);
            
            root.Bind(serializedObject);

            var set = (StepJointSet)target;

            ListView joints = root.Q<ListView>("joints-list");
            joints.makeItem = MakeItem;
            joints.bindItem = (element, index) => ((Label)element).text = JointText(set.joints[index]);
            joints.itemsSource = new List<StepJoint>(set.joints);
            joints.Rebuild();
            // Unity 6 passes the selection (possibly several entries) to
            // selectionChanged; the set overview acts on the first one.
            joints.selectionChanged += items => SelectJoint(items.FirstOrDefault() as StepJoint);

            ListView actuators = root.Q<ListView>("actuators-list");
            actuators.makeItem = MakeItem;
            actuators.bindItem = (element, index) => ((Label)element).text = ActuatorText(set.actuators[index]);
            actuators.itemsSource = new List<StepActuator>(set.actuators);
            actuators.Rebuild();
            actuators.selectionChanged += items => SelectActuator(items.FirstOrDefault() as StepActuator);

            ListView geoms = root.Q<ListView>("geoms-list");
            geoms.makeItem = MakeItem;
            geoms.bindItem = (element, index) => ((Label)element).text = GeomText(set.geoms[index]);
            geoms.itemsSource = new List<StepGeom>(set.geoms);
            geoms.Rebuild();
            geoms.selectionChanged += items => SelectGeom(items.FirstOrDefault() as StepGeom);

            root.Q<Button>("export-button").clicked += () => StepMjcfExporter.Export(set);

            root.RegisterCallback<SerializedPropertyChangeEvent>(change => RebuildLists(joints, actuators, geoms));
            return root;
        }

        private void RebuildLists(ListView joints, ListView actuators, ListView geoms)
        {
            var set = (StepJointSet)target;
            if (set == null)
            {
                return;
            }
            joints.itemsSource = new List<StepJoint>(set.joints);
            actuators.itemsSource = new List<StepActuator>(set.actuators);
            geoms.itemsSource = new List<StepGeom>(set.geoms);
            joints.Rebuild();
            actuators.Rebuild();
            geoms.Rebuild();
        }

        private static VisualElement MakeItem()
        {
            var label = new Label();
            label.AddToClassList("list-item");
            return label;
        }

        private static string JointText(StepJoint joint)
        {
            return joint != null ? $"{joint.name}  ({joint.mj.type})" : "(missing joint)";
        }

        private static string ActuatorText(StepActuator actuator)
        {
            return actuator != null
                ? $"{actuator.name}  →  {(actuator.target != null ? actuator.target.name : "(no target)")}"
                : "(missing actuator)";
        }

        private static string GeomText(StepGeom geom)
        {
            return geom != null ? $"{geom.name}  ({geom.mj.type})" : "(missing geom)";
        }

        private static void SelectGeom(StepGeom geom)
        {
            if (geom == null)
            {
                return;
            }
            SelectAsset(geom);
            GameObject part = StepJointSetFactory.FindPart(geom.root, geom.body);
            if (part != null)
            {
                EditorGUIUtility.PingObject(part);
            }
        }

        private static void SelectJoint(StepJoint joint)
        {
            if (joint == null)
            {
                return;
            }
            SelectAsset(joint);
            GameObject part = StepJointSetFactory.FindPart(joint);
            if (part != null)
            {
                EditorGUIUtility.PingObject(part);
            }
        }

        private static void SelectActuator(StepActuator actuator)
        {
            if (actuator == null)
            {
                return;
            }
            SelectAsset(actuator);
            if (actuator.target != null)
            {
                GameObject part = StepJointSetFactory.FindPart(actuator.target);
                if (part != null)
                {
                    EditorGUIUtility.PingObject(part);
                }
            }
        }

        private static void SelectAsset(Object asset)
        {
            Selection.activeObject = asset;
            EditorGUIUtility.PingObject(asset);
        }
    }
}
