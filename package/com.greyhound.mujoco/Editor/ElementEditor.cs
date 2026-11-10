using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine.UIElements;

namespace Greyhound.MuJoCo
{
    // Base inspector for the element assets (Joint, Actuator, Site, Geom,
    // Sensor): the default fields, plus one rule. Whenever the payload's
    // MJCF `name` changes to a non-empty value, the asset is renamed to
    // match, so the Project window shows the MJCF name. An empty name
    // leaves the asset's name alone. The name field is [Delayed], so the
    // rename happens on Enter or when the field loses focus, not on every
    // keystroke. Undo restores both.
    //
    // NamePath is the payload's name property, e.g. "joint.name".
    // Subclasses add their own rules in Extend.
    internal abstract class ElementEditor : Editor
    {
        protected abstract string NamePath { get; }

        public override VisualElement CreateInspectorGUI()
        {
            VisualElement container = BuildLayout();
            container.TrackPropertyValue(serializedObject.FindProperty(NamePath), SyncAssetName);
            Extend(container);
            return container;
        }

        // The inspector's fields. Default: Unity's default inspector. A
        // subclass with its own UXML layout overrides this.
        protected virtual VisualElement BuildLayout()
        {
            var container = new VisualElement();
            InspectorElement.FillDefaultInspector(container, serializedObject, this);
            return container;
        }

        protected virtual void Extend(VisualElement container)
        {
        }

        // Sets the label of the PropertyField named `name` to
        // "<attribute> (<unit>)", or just the attribute when the unit is
        // empty (unknown, or not applicable). Used for units that change
        // with other fields; fixed units are written in the UXML labels.
        protected static void SetLabel(VisualElement root, string name, string attribute, string unit)
        {
            PropertyField field = root.Q<PropertyField>(name);
            if (field != null)
            {
                field.label = string.IsNullOrEmpty(unit) ? attribute : $"{attribute} ({unit})";
            }
        }

        // Fills an empty MJCF name from a body path that has a root and a
        // target (indices not empty; [SELF] = [-1] is the root itself):
        // "<prefix>[<indices>]-<targetName>", indices comma-separated.
        // Example: prefix "j-", path [2,0] on part "Pole" -> "j-[2,0]-Pole".
        // A name already set, typed or generated, is never replaced.
        // Setting it renames the asset through SyncAssetName.
        protected void FillName(string prefix, SiblingPath path)
        {
            if (path == null || path.root == null || path.indices == null || path.indices.Length == 0)
            {
                return;
            }
            serializedObject.Update();
            SerializedProperty name = serializedObject.FindProperty(NamePath);
            if (!string.IsNullOrEmpty(name.stringValue))
            {
                return;
            }
            name.stringValue = $"{prefix}[{string.Join(",", path.indices ?? new int[0])}]-{path.targetName}";
            serializedObject.ApplyModifiedProperties();
        }

        private void SyncAssetName(SerializedProperty name)
        {
            string value = name.stringValue;
            if (string.IsNullOrEmpty(value) || target == null || target.name == value)
            {
                return;
            }
            Undo.RecordObject(target, "Rename Element");
            target.name = value;
            EditorUtility.SetDirty(target);
            AssetDatabase.SaveAssetIfDirty(target);
        }
    }
}
