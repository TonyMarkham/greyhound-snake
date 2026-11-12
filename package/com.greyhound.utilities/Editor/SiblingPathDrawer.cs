using System.Collections.Generic;

using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace Greyhound.Utilities
{
    // Draws a SiblingPath: an assignable root field, then the live chain
    // from an instance of root down to the target (one row per level,
    // the live name plus the stored index that reached it), then the
    // name stored when the path was created (the first drop). The target
    // is set only by the drop zone, which takes a GameObject from the
    // scene.
    //
    // Root rule: the dropped GameObject and its ancestors are walked
    // upward, and the first whose original source is the main asset of
    // an asset file is the instance root; that main asset is `root`.
    // Example: a part of an imported arm.step instance walks up to the
    // instance root, whose source is arm.step's root GameObject. `root`
    // is always an asset because the holder may be a ScriptableObject,
    // which cannot reference scene objects.
    [CustomPropertyDrawer(typeof(SiblingPath))]
    internal sealed class SiblingPathDrawer : PropertyDrawer
    {
        private const string STYLE_SHEET_PATH = "Packages/com.greyhound.utilities/Editor/UI/SiblingPath.uss";

        public override VisualElement CreatePropertyGUI(SerializedProperty property)
        {
            var container = new VisualElement();
            container.AddToClassList("sibling-path-card");
            var styleSheet = AssetDatabase.LoadAssetAtPath<StyleSheet>(STYLE_SHEET_PATH);
            if (styleSheet != null)
            {
                container.styleSheets.Add(styleSheet);
            }

            var foldout = new Foldout { value = false };
            container.Add(foldout);

            // Assets only: the holder may be a ScriptableObject, which
            // cannot reference scene objects.
            var rootField = new ObjectField("root") { objectType = typeof(GameObject), allowSceneObjects = false };
            rootField.BindProperty(property.FindPropertyRelative(nameof(SiblingPath.root)));
            foldout.Add(rootField);

            // Indices only mean something under a root, so setting root to
            // None clears them. root is written here too, so the clear
            // never depends on whether the binding has applied it yet.
            rootField.RegisterValueChangedCallback(change =>
            {
                SerializedProperty indices = property.FindPropertyRelative(nameof(SiblingPath.indices));
                if (change.newValue != null || indices.arraySize == 0)
                {
                    return;
                }
                property.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue = null;
                indices.arraySize = 0;
                property.serializedObject.ApplyModifiedProperties();
            });

            var rows = new VisualElement();
            rows.AddToClassList("sibling-path-chain");
            foldout.Add(rows);

            var dropZone = new VisualElement();
            dropZone.AddToClassList("sibling-path-dropzone");
            dropZone.Add(new Label("drop a scene GameObject to set the target"));
            dropZone.generateVisualContent += DrawDashedBorder;
            dropZone.RegisterCallback<GeometryChangedEvent>(_ => dropZone.MarkDirtyRepaint());
            container.Add(dropZone);

            // Pings the target GameObject in the Hierarchy. The path is
            // resolved at click time against the first instance of root in
            // the open scenes, so it works however the scene changed since
            // the inspector was built; with no instance open, no target, or
            // a path that no longer fits, nothing is pinged and the Console
            // says why.
            var ping = new Button { text = "Ping" };
            ping.AddToClassList("sibling-path-ping");
            ping.clicked += () =>
            {
                var root = (GameObject)property.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue;
                int[] indices = ReadIndices(property);
                List<Transform> chain = ResolveChain(root, indices);
                Transform target = indices.Length > 0 && chain.Count > 0 ? chain[chain.Count - 1] : null;
                if (target != null)
                {
                    EditorGUIUtility.PingObject(target.gameObject);
                    return;
                }
                string reason = root == null ? "it has no root"
                    : indices.Length == 0 ? "it has no target"
                    : chain.Count == 0 ? $"no instance of '{root.name}' is in the open scenes"
                    : "its path no longer fits the hierarchy";
                Debug.LogWarning($"{property.serializedObject.targetObject.name}: cannot ping the target, {reason}.",
                    property.serializedObject.targetObject);
            };
            container.Add(ping);

            // Built once; only the contents refresh, so the foldout keeps
            // its open state across refreshes.
            void Refresh()
            {
                var root = (GameObject)property.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue;
                int[] indices = ReadIndices(property);
                string storedName = property.FindPropertyRelative(nameof(SiblingPath.targetName)).stringValue;

                // Bar colour: red = no root; yellow = root but no target
                // (indices []); green = root and a target, the root itself
                // included (indices [SELF] = [-1]).
                bool targeted = indices.Length > 0;
                container.EnableInClassList("sibling-path-card--empty", root == null);
                container.EnableInClassList("sibling-path-card--rooted", root != null && !targeted);
                container.EnableInClassList("sibling-path-card--set", root != null && targeted);

                List<Transform> chain = ResolveChain(root, indices);
                foldout.text = $"{property.displayName}: {Headline(root, chain, targeted)}";

                rows.Clear();
                for (int i = 0; i < chain.Count; i++)
                {
                    string nodeName = chain[i] != null ? chain[i].name : "(missing)";
                    var row = new Label(i == 0 ? nodeName : $"{nodeName}  [{indices[i - 1]}]");
                    row.AddToClassList(targeted && i == chain.Count - 1
                        ? "sibling-path-chain__row--target"
                        : "sibling-path-chain__row");
                    rows.Add(row);
                }
                // The resolved name is never serialized: it is read from the
                // live target on every refresh, so it follows renames that
                // the stored creation-time Target Name does not.
                Transform resolved = targeted && chain.Count > 0 ? chain[chain.Count - 1] : null;
                var names = new VisualElement();
                names.AddToClassList("sibling-path-names");
                var nameRow = new Label($"Target Name: {(string.IsNullOrEmpty(storedName) ? "(none)" : storedName)}");
                nameRow.AddToClassList("sibling-path-chain__row");
                names.Add(nameRow);
                var resolvedRow = new Label($"Resolved Name: {(resolved != null ? resolved.name : targeted ? "(unresolved)" : "(no target)")}");
                resolvedRow.AddToClassList("sibling-path-chain__row");
                names.Add(resolvedRow);
                rows.Add(names);
            }

            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = TryGetDrop(property, out _, out _, out _)
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                if (!TryGetDrop(property, out Transform target, out Transform instanceRoot, out GameObject rootAsset))
                {
                    return;
                }
                WritePath(property, rootAsset, SiblingPath.IndicesOf(target, instanceRoot), target.name);
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
                Refresh();
            });

            // Redraws on changes made outside the drop zone, such as an
            // undo of a drop.
            container.TrackPropertyValue(property, _ => Refresh());

            Refresh();
            return container;
        }

        // Dashed outline values: 4px dash, 3px gap, 3px corner radius, the
        // grey the solid border used. Each corner is approximated by 6
        // straight segments, enough to look round at a 3px radius.
        private const float DASH_LENGTH = 4f;
        private const float DASH_GAP = 3f;
        private const float DASH_CORNER_RADIUS = 3f;
        private const int DASH_CORNER_SEGMENTS = 6;
        private static readonly Color k_DashColor = new Color(90f / 255f, 90f / 255f, 90f / 255f);

        // Painter2D has no dash setting, so the rounded rectangle is
        // flattened into a closed polyline and walked once, alternating
        // DASH_LENGTH drawn and DASH_GAP skipped; a dash may run on
        // around a corner. The outline is inset half a line width so the
        // 1px stroke lands inside the element.
        private static void DrawDashedBorder(MeshGenerationContext context)
        {
            Vector2 size = context.visualElement.layout.size;
            var rect = new Rect(0.5f, 0.5f, size.x - 1f, size.y - 1f);
            if (rect.width <= 0f || rect.height <= 0f)
            {
                return;
            }

            List<Vector2> outline = RoundedRectOutline(rect, DASH_CORNER_RADIUS, DASH_CORNER_SEGMENTS);

            Painter2D painter = context.painter2D;
            painter.strokeColor = k_DashColor;
            painter.lineWidth = 1f;
            painter.BeginPath();

            bool drawing = true;
            float remaining = DASH_LENGTH;
            painter.MoveTo(outline[0]);
            for (int i = 0; i < outline.Count - 1; i++)
            {
                Vector2 from = outline[i];
                Vector2 to = outline[i + 1];
                float length = Vector2.Distance(from, to);
                float travelled = 0f;
                while (travelled < length)
                {
                    float step = Mathf.Min(remaining, length - travelled);
                    travelled += step;
                    remaining -= step;
                    Vector2 point = Vector2.Lerp(from, to, travelled / length);
                    if (drawing)
                    {
                        painter.LineTo(point);
                    }
                    if (remaining <= 0f)
                    {
                        drawing = !drawing;
                        remaining = drawing ? DASH_LENGTH : DASH_GAP;
                        if (drawing)
                        {
                            painter.MoveTo(point);
                        }
                    }
                }
            }
            painter.Stroke();
        }

        // Clockwise in UI Toolkit coordinates (y down), starting on the
        // top-left arc, with the first point repeated at the end to close
        // it. Arc angles: top-left 180°→270°, top-right 270°→360°,
        // bottom-right 0°→90°, bottom-left 90°→180°.
        private static List<Vector2> RoundedRectOutline(Rect rect, float radius, int segments)
        {
            radius = Mathf.Min(radius, rect.width * 0.5f, rect.height * 0.5f);
            var points = new List<Vector2>();
            AddArc(points, new Vector2(rect.xMin + radius, rect.yMin + radius), radius, 180f, segments);
            AddArc(points, new Vector2(rect.xMax - radius, rect.yMin + radius), radius, 270f, segments);
            AddArc(points, new Vector2(rect.xMax - radius, rect.yMax - radius), radius, 0f, segments);
            AddArc(points, new Vector2(rect.xMin + radius, rect.yMax - radius), radius, 90f, segments);
            points.Add(points[0]);
            return points;
        }

        private static void AddArc(List<Vector2> points, Vector2 center, float radius, float startDegrees, int segments)
        {
            for (int i = 0; i <= segments; i++)
            {
                float radians = (startDegrees + 90f * i / segments) * Mathf.Deg2Rad;
                points.Add(center + new Vector2(Mathf.Cos(radians), Mathf.Sin(radians)) * radius);
            }
        }

        private static string Headline(GameObject root, List<Transform> chain, bool targeted)
        {
            if (root == null)
            {
                return "(no root)";
            }
            if (!targeted)
            {
                return "(no target)";
            }
            if (chain.Count == 0)
            {
                return "(no instance of root in the open scenes)";
            }
            Transform target = chain[chain.Count - 1];
            return target != null ? target.name : "(unresolved)";
        }

        // Valid when a scene GameObject is dragged and an instance root is
        // found for it, walking from the dropped object up through its
        // ancestors. With `root` empty, the instance root is the first
        // whose original source is a main asset. With `root` set, an
        // instance of `root` must be the dropped object itself or one of
        // its ancestors; otherwise the drop is rejected. Project-window
        // assets are skipped: a path indexes a live hierarchy.
        private static bool TryGetDrop(
            SerializedProperty property, out Transform target, out Transform instanceRoot, out GameObject rootAsset)
        {
            target = null;
            instanceRoot = null;
            rootAsset = (GameObject)property.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue;

            foreach (Object reference in DragAndDrop.objectReferences)
            {
                if (reference is not GameObject candidate || EditorUtility.IsPersistent(candidate))
                {
                    continue;
                }
                for (Transform node = candidate.transform; node != null; node = node.parent)
                {
                    GameObject source = PrefabUtility.GetCorrespondingObjectFromOriginalSource(node.gameObject);
                    bool isRoot = rootAsset != null
                        ? source == rootAsset
                        : source != null && AssetDatabase.IsMainAsset(source);
                    if (isRoot)
                    {
                        target = candidate.transform;
                        instanceRoot = node;
                        rootAsset = source;
                        return true;
                    }
                }
            }
            return false;
        }

        // Root first, target last; once an index falls outside its
        // parent's child count, that position and every later one is
        // null. Empty when root is unset or has no instance in the
        // loaded scenes (the first instance in hierarchy order is used).
        private static List<Transform> ResolveChain(GameObject root, int[] indices)
        {
            var chain = new List<Transform>();
            Transform node = root != null ? SiblingPath.FindInstance(root, matchByName: false) : null;
            if (node == null)
            {
                return chain;
            }
            chain.Add(node);
            if (SiblingPath.IsSelf(indices))
            {
                return chain;
            }
            foreach (int index in indices)
            {
                node = node != null && index >= 0 && index < node.childCount
                    ? node.GetChild(index)
                    : null;
                chain.Add(node);
            }
            return chain;
        }


        private static int[] ReadIndices(SerializedProperty property)
        {
            SerializedProperty indices = property.FindPropertyRelative(nameof(SiblingPath.indices));
            var result = new int[indices.arraySize];
            for (int i = 0; i < result.Length; i++)
            {
                result[i] = indices.GetArrayElementAtIndex(i).intValue;
            }
            return result;
        }

        private static void WritePath(SerializedProperty property, GameObject rootAsset, int[] path, string targetName)
        {
            property.FindPropertyRelative(nameof(SiblingPath.root)).objectReferenceValue = rootAsset;
            SerializedProperty indices = property.FindPropertyRelative(nameof(SiblingPath.indices));
            indices.arraySize = path.Length;
            for (int i = 0; i < path.Length; i++)
            {
                indices.GetArrayElementAtIndex(i).intValue = path[i];
            }
            // targetName is the creation-time name: written by the first
            // drop onto a path with no targetName, never overwritten.
            SerializedProperty storedName = property.FindPropertyRelative(nameof(SiblingPath.targetName));
            if (string.IsNullOrEmpty(storedName.stringValue))
            {
                storedName.stringValue = targetName;
            }
            property.serializedObject.ApplyModifiedProperties();
        }
    }
}
