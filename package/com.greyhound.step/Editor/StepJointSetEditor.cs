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

            ListView sites = root.Q<ListView>("sites-list");
            sites.makeItem = MakeItem;
            sites.bindItem = (element, index) => ((Label)element).text = SiteText(set.sites[index]);
            sites.itemsSource = new List<StepSite>(set.sites);
            sites.Rebuild();
            sites.selectionChanged += items => SelectSite(items.FirstOrDefault() as StepSite);
            // Site assets dropped on the list join the set (a site made
            // from the Create menu starts in none): one with no Root is
            // given this set; one already in this set, or owned by
            // another set, is refused.
            sites.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = DroppedSites(set).Count > 0
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            sites.RegisterCallback<DragPerformEvent>(drag =>
            {
                List<StepSite> dropped = DroppedSites(set);
                if (dropped.Count == 0)
                {
                    return;
                }
                Undo.RecordObject(set, "Add Step Site");
                foreach (StepSite site in dropped)
                {
                    if (site.root == null)
                    {
                        Undo.RecordObject(site, "Add Step Site");
                        site.root = set;
                        EditorUtility.SetDirty(site);
                    }
                    set.sites.Add(site);
                }
                EditorUtility.SetDirty(set);
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
                sites.itemsSource = new List<StepSite>(set.sites);
                sites.Rebuild();
            });

            ListView sensors = root.Q<ListView>("sensors-list");
            sensors.makeItem = MakeItem;
            sensors.bindItem = (element, index) => ((Label)element).text = SensorText(set.sensors[index]);
            sensors.itemsSource = new List<StepSensor>(set.sensors);
            sensors.Rebuild();
            sensors.selectionChanged += items => SelectSensor(items.FirstOrDefault() as StepSensor);

            root.Q<Button>("export-button").clicked += () => StepMjcfExporter.Export(set);

            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
                RebuildLists(joints, actuators, geoms, sites, sensors));
            return root;
        }

        private void RebuildLists(
            ListView joints, ListView actuators, ListView geoms, ListView sites, ListView sensors)
        {
            var set = (StepJointSet)target;
            if (set == null)
            {
                return;
            }
            joints.itemsSource = new List<StepJoint>(set.joints);
            actuators.itemsSource = new List<StepActuator>(set.actuators);
            geoms.itemsSource = new List<StepGeom>(set.geoms);
            sites.itemsSource = new List<StepSite>(set.sites);
            sensors.itemsSource = new List<StepSensor>(set.sensors);
            joints.Rebuild();
            actuators.Rebuild();
            geoms.Rebuild();
            sites.Rebuild();
            sensors.Rebuild();
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

        private static string SiteText(StepSite site)
        {
            return site != null
                ? $"{site.name}  ({(site.body != null ? site.body.name : "no part")})"
                : "(missing site)";
        }

        private static string SensorText(StepSensor sensor)
        {
            if (sensor == null)
            {
                return "(missing sensor)";
            }
            string target = sensor.target != null ? sensor.target.name : "(no target)";
            string reference = sensor.reference != null ? sensor.reference.name : "world";
            return $"{sensor.name}  →  {target}  from {reference}";
        }

        private static List<StepSite> DroppedSites(StepJointSet set)
        {
            return DragAndDrop.objectReferences
                .OfType<StepSite>()
                .Where(site => !set.sites.Contains(site) && (site.root == null || site.root == set))
                .Distinct()
                .ToList();
        }

        private static void SelectSite(StepSite site)
        {
            if (site == null)
            {
                return;
            }
            SelectAsset(site);
            GameObject part = StepJointSetFactory.FindPart(site.root, site.body);
            if (part != null)
            {
                EditorGUIUtility.PingObject(part);
            }
        }

        private static void SelectSensor(StepSensor sensor)
        {
            if (sensor == null)
            {
                return;
            }
            SelectAsset(sensor);
            if (sensor.target != null)
            {
                GameObject part = StepJointSetFactory.FindPart(sensor.target.root, sensor.target.body);
                if (part != null)
                {
                    EditorGUIUtility.PingObject(part);
                }
            }
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
