using System.Collections.Generic;
using System.IO;
using System.Linq;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Root inspector. Layout in UI/Root.uxml, styles in UI/Root.uss,
    // both assigned as this script's default references (its .meta), as
    // for the step package's editors. Top to bottom:
    //   - Model card: the model asset and rootMobility. Bar red with no
    //     model, green with one.
    //   - Simulation (open): timestep and integrator, the two <option>
    //     attributes always exported.
    //   - Physics, Solver, Contact override, Flags, Actuator groups
    //     (collapsed): the rest of <option>.
    //   - Element cards: the joint, actuator, site, geom and sensor lists,
    //     one card each with a type-coloured bar, each labelled with its
    //     count. They stay editable: they are how elements join this Root.
    //   - Export: the button, then the result (written XML path, mesh
    //     count, Show in Explorer) or the refusal.
    // Labels are the MJCF attribute names, verbatim.
    [CustomEditor(typeof(SO.Root))]
    internal sealed class RootEditor : Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(RootEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(RootEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            // Element lists open collapsed each time the inspector is
            // built. A list PropertyField draws its foldout from the
            // property's isExpanded, so it is reset before binding.
            foreach (string list in new[] { "joints", "actuators", "sites", "geoms", "sensors" })
            {
                serializedObject.FindProperty(list).isExpanded = false;
            }

            root.Bind(serializedObject);

            BindModelCard(root.Q<VisualElement>("model-card"));
            CountInLabel(root.Q<PropertyField>("joints-list"), "joints", "Joints");
            CountInLabel(root.Q<PropertyField>("actuators-list"), "actuators", "Actuators");
            CountInLabel(root.Q<PropertyField>("sites-list"), "sites", "Sites");
            CountInLabel(root.Q<PropertyField>("geoms-list"), "geoms", "Geoms");
            CountInLabel(root.Q<PropertyField>("sensors-list"), "sensors", "Sensors");
            root.Q<Button>("joints-add").clicked += () => AddElement<SO.Joint>("joints", "Joint");
            root.Q<Button>("actuators-add").clicked += () => AddElement<SO.Actuator>("actuators", "Actuator");
            root.Q<Button>("sites-add").clicked += () => AddElement<SO.Site>("sites", "Site");
            root.Q<Button>("geoms-add").clicked += () => AddElement<SO.Geom>("geoms", "Geom");
            root.Q<Button>("sensors-add").clicked += () => AddElement<SO.Sensor>("sensors", "Sensor");
            foreach (string list in new[] { "joints", "actuators", "sites", "geoms", "sensors" })
            {
                root.TrackPropertyValue(serializedObject.FindProperty(list), _ => RemoveUnlisted());
            }
            BindExport(root);
            return root;
        }

        // Removing an element from its list (the list's - button, or
        // clearing the slot) also deletes it from the Root's .asset file:
        // after any list change, every element sub-asset of this file that
        // no list references is destroyed. Only Joint, Actuator, Site, Geom
        // and Sensor sub-assets are touched, and only in this Root's file.
        // The delete is its own undo step: one undo brings the asset back,
        // a second undo puts it back in the list.
        private void RemoveUnlisted()
        {
            var rootAsset = (SO.Root)target;
            if (rootAsset == null)
            {
                return;
            }
            string assetPath = AssetDatabase.GetAssetPath(rootAsset);
            if (string.IsNullOrEmpty(assetPath))
            {
                return;
            }

            var listed = new HashSet<Object>();
            void Collect<T>(List<T> list) where T : Object
            {
                if (list == null)
                {
                    return;
                }
                foreach (T element in list)
                {
                    if (element != null)
                    {
                        listed.Add(element);
                    }
                }
            }
            Collect(rootAsset.joints);
            Collect(rootAsset.actuators);
            Collect(rootAsset.sites);
            Collect(rootAsset.geoms);
            Collect(rootAsset.sensors);

            bool removed = false;
            foreach (Object asset in AssetDatabase.LoadAllAssetsAtPath(assetPath))
            {
                bool element = asset is SO.Joint || asset is SO.Actuator || asset is SO.Site
                    || asset is SO.Geom || asset is SO.Sensor;
                if (element && !listed.Contains(asset))
                {
                    Undo.DestroyObjectImmediate(asset);
                    removed = true;
                }
            }
            if (removed)
            {
                EditorUtility.SetDirty(rootAsset);
                AssetDatabase.SaveAssetIfDirty(rootAsset);
            }
        }

        // Creates a new element as a sub-asset of this Root's .asset file,
        // so it shows as a child of the Root in the Project window. Its body
        // path (if it has one) starts on this Root's model, and it is
        // appended to the matching list. The asset is named after its type,
        // made unique among the file's assets ("Joint", "Joint 1", ...);
        // the MJCF name is the payload's `name`, or this asset name when
        // that is empty.
        private void AddElement<T>(string listPath, string baseName) where T : ScriptableObject
        {
            var rootAsset = (SO.Root)target;
            string assetPath = AssetDatabase.GetAssetPath(rootAsset);
            if (string.IsNullOrEmpty(assetPath))
            {
                Debug.LogError($"{rootAsset.name}: save the Root as an asset before adding elements.", rootAsset);
                return;
            }

            string[] taken = AssetDatabase.LoadAllAssetsAtPath(assetPath)
                .Where(asset => asset != null)
                .Select(asset => asset.name)
                .ToArray();
            var element = CreateInstance<T>();
            element.name = ObjectNames.GetUniqueName(taken, baseName);
            AssetDatabase.AddObjectToAsset(element, rootAsset);
            Undo.RegisterCreatedObjectUndo(element, $"Add {baseName}");

            var elementObject = new SerializedObject(element);
            // An element on a part (Joint, Site, Geom) starts with its
            // body path on this Root's model, so a drop only has to pick
            // the part. Actuator and Sensor have no body path.
            SerializedProperty body = elementObject.FindProperty("body");
            if (body != null)
            {
                body.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue = rootAsset.root;
            }
            elementObject.ApplyModifiedPropertiesWithoutUndo();

            serializedObject.Update();
            SerializedProperty list = serializedObject.FindProperty(listPath);
            list.arraySize++;
            list.GetArrayElementAtIndex(list.arraySize - 1).objectReferenceValue = element;
            list.isExpanded = true;
            serializedObject.ApplyModifiedProperties();

            AssetDatabase.SaveAssetIfDirty(rootAsset);
        }

        private void BindModelCard(VisualElement card)
        {
            SerializedProperty model = serializedObject.FindProperty("root");
            void Refresh(SerializedProperty property)
            {
                bool set = property.objectReferenceValue != null;
                card.EnableInClassList("mujoco-card--set", set);
                card.EnableInClassList("mujoco-card--empty", !set);
            }
            Refresh(model);
            card.TrackPropertyValue(model, Refresh);
        }

        private void CountInLabel(PropertyField field, string path, string label)
        {
            SerializedProperty list = serializedObject.FindProperty(path);
            void Relabel(SerializedProperty property)
            {
                field.label = $"{label} ({property.arraySize})";
            }
            Relabel(list);
            field.TrackPropertyValue(list, Relabel);
        }

        private void BindExport(VisualElement root)
        {
            var result = new HelpBox(string.Empty, HelpBoxMessageType.Info);
            result.style.display = DisplayStyle.None;
            root.Q<VisualElement>("export-result").Add(result);
            Button reveal = root.Q<Button>("reveal-button");
            string lastXml = null;

            root.Q<Button>("export-button").clicked += () =>
            {
                var asset = (SO.Root)target;
                string folder = EditorUtility.OpenFolderPanel("Export MJCF", StartFolder(asset), string.Empty);
                if (string.IsNullOrEmpty(folder))
                {
                    return;
                }
                List<string> written = MjcfExporter.Export(asset, folder);
                result.style.display = DisplayStyle.Flex;
                if (written == null)
                {
                    result.messageType = HelpBoxMessageType.Error;
                    result.text = "Export refused; nothing was written. The reasons are in the Console.";
                    reveal.style.display = DisplayStyle.None;
                    return;
                }
                lastXml = Path.GetFullPath(written[0]);
                result.messageType = HelpBoxMessageType.Info;
                result.text = $"Wrote {lastXml}\nand {written.Count - 1} mesh file(s) in {Path.Combine(Path.GetDirectoryName(lastXml), "meshes")}";
                reveal.style.display = DisplayStyle.Flex;
            };
            reveal.clicked += () =>
            {
                if (lastXml != null)
                {
                    EditorUtility.RevealInFinder(lastXml);
                }
            };
        }

        // The model asset's folder, or the project's Assets folder when the
        // Root has no model yet.
        private static string StartFolder(SO.Root root)
        {
            string assetPath = root.root != null ? AssetDatabase.GetAssetPath(root.root) : null;
            string directory = string.IsNullOrEmpty(assetPath) ? "Assets" : Path.GetDirectoryName(assetPath);
            return Path.GetFullPath(directory);
        }
    }
}
