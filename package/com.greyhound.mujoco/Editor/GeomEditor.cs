using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

using SO = Greyhound.MuJoCo.ScriptableObjects;

namespace Greyhound.MuJoCo
{
    // The Geom inspector. Layout in UI/Geom.uxml, styles in UI/Geom.uss,
    // both assigned as this script's default references (its .meta), as
    // for JointEditor. Top to bottom: the identity card (name, body,
    // override default, type) with the Geoms bar colour, Placement and
    // Shape (open), then Contact, Mass, Appearance, Fluid and Other
    // (collapsed). Labels are the MJCF attribute names.
    //
    // Shown only when they apply (values stay stored either way):
    //   - every section ("geom-sections") unless type is Null: a Null geom
    //     writes nothing, so only the identity card is shown. Selecting
    //     Null also checks override default: a Null geom always overrides.
    //   - pos, quat ("frame-only") while useFromto is off; fromto from/to
    //     ("fromto-only") while it is on.
    //   - mesh ("mesh-only") while type is Mesh; hfield ("hfield-only")
    //     while type is Hfield.
    //
    // Naming: when the body path changes and has a root, an empty MJCF
    // `name` becomes "g-[<indices>]-<targetName>", e.g. "g-[2,0]-Pole". A
    // name already set is never replaced. Setting the name renames the
    // asset to match (ElementEditor).
    [CustomEditor(typeof(SO.Geom))]
    internal sealed class GeomEditor : ElementEditor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;
        [SerializeField] private StyleSheet m_StyleSheet;

        protected override string NamePath => "geom.name";

        protected override VisualElement BuildLayout()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(GeomEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(GeomEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            VisualElement sections = root.Q<VisualElement>("geom-sections");
            SerializedProperty type = serializedObject.FindProperty("geom.type");
            void ShowForType(SerializedProperty property)
            {
                var value = (GeomType)property.enumValueIndex;
                sections.style.display = value == GeomType.Null ? DisplayStyle.None : DisplayStyle.Flex;
                // A Null geom always overrides (the exporter enforces it
                // regardless); the toggle is checked so the inspector shows
                // what the export will do.
                SerializedProperty overrideDefault = serializedObject.FindProperty("overrideDefault");
                if (value == GeomType.Null && !overrideDefault.boolValue)
                {
                    overrideDefault.boolValue = true;
                    serializedObject.ApplyModifiedProperties();
                }
                Show(root, "mesh-only", value == GeomType.Mesh);
                Show(root, "hfield-only", value == GeomType.Hfield);
            }
            ShowForType(type);
            root.TrackPropertyValue(type, ShowForType);

            SerializedProperty useFromto = serializedObject.FindProperty("geom.useFromto");
            void ShowForFromto(SerializedProperty property)
            {
                Show(root, "frame-only", !property.boolValue);
                Show(root, "fromto-only", property.boolValue);
            }
            ShowForFromto(useFromto);
            root.TrackPropertyValue(useFromto, ShowForFromto);

            // density is mass per volume (kg/m³) for a solid geom and mass
            // per area (kg/m²) when shellinertia is on (XML reference,
            // geom/density). Every other unit is fixed in the UXML.
            SerializedProperty shellinertia = serializedObject.FindProperty("geom.shellinertia");
            void RefreshDensity(SerializedProperty property)
            {
                SetLabel(root, "unit-density", "density", property.boolValue ? "kg/m²" : "kg/m³");
            }
            RefreshDensity(shellinertia);
            root.TrackPropertyValue(shellinertia, RefreshDensity);
            return root;
        }

        protected override void Extend(VisualElement container)
        {
            container.TrackPropertyValue(
                serializedObject.FindProperty("body"), _ => FillName("g-", ((SO.Geom)target).body));
        }

        private static void Show(VisualElement root, string className, bool visible)
        {
            root.Query<VisualElement>(className: className)
                .ForEach(field => field.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None);
        }
    }
}
