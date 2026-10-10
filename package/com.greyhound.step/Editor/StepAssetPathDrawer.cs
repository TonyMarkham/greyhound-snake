using System.IO;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Step
{
    // Renders the serialized .stp path as an asset link to the actual
    // file. The path string stays the serialized store: a plain object
    // reference would hang off the imported artifact's file IDs and
    // dangle on reimport, while the project path survives every
    // reimport (and a move of the .stp together with its folder).
    [CustomPropertyDrawer(typeof(StepAssetPathAttribute))]
    internal sealed class StepAssetPathDrawer : PropertyDrawer
    {
        public override VisualElement CreatePropertyGUI(SerializedProperty property)
        {
            var link = new ObjectField(property.displayName)
            {
                objectType = typeof(Object),
                allowSceneObjects = false,
            };

            string path = property.stringValue;
            link.value = string.IsNullOrEmpty(path)
                ? null
                : AssetDatabase.LoadAssetAtPath<Object>(path);

            link.RegisterValueChangedCallback(change =>
            {
                Object picked = change.newValue;
                string pickedPath = picked != null ? AssetDatabase.GetAssetPath(picked) : "";
                string extension = Path.GetExtension(pickedPath).ToLowerInvariant();
                if (picked == null || extension == ".stp" || extension == ".step")
                {
                    property.stringValue = pickedPath;
                    property.serializedObject.ApplyModifiedProperties();
                }
                else
                {
                    Debug.LogError($"{pickedPath}: not a STEP file (.stp/.step)");
                }
            });

            return link;
        }
    }
}
