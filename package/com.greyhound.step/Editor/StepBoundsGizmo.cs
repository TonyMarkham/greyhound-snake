using Unity.Scripting.LifecycleManagement;
using UnityEditor;
using UnityEngine;

namespace Greyhound.Step
{
    // Editor-only diagnostic for the imported STEP hierarchy: draws the
    // meshes' bounds for whatever is selected. Toggle via
    // Tools > Step Bounds Gizmo. Yellow = the mesh-local bounds the host
    // emits (oriented by each part's transform); cyan = Unity's world-space
    // renderer bounds (what selection framing and the Center handle use);
    // RGB axes = the selection's pivot; magenta = pivot-to-center link.
    // Tools > Step/Log Bounds prints each mesh's emitted bounds next to
    // Unity's renderer bounds and a fresh RecalculateBounds result.
    [InitializeOnLoad]
    [NoAutoStaticsCleanup]
    internal static class StepBoundsGizmo
    {
        private const string MenuPath = "Tools/Step Bounds Gizmo";

        private static bool enabled;

        private static readonly Color MeshBoundsColor = new Color(1f, 0.9f, 0.2f);
        private static readonly Color RendererBoundsColor = new Color(0.2f, 0.9f, 1f);
        private static readonly Color CenterLinkColor = new Color(1f, 0.2f, 1f);

        static StepBoundsGizmo()
        {
            SceneView.duringSceneGui += OnSceneGui;
        }

        [MenuItem("Tools/Step/Log Bounds")]
        private static void LogBounds()
        {
            foreach (Transform selection in Selection.transforms)
            {
                foreach (MeshFilter filter in selection.GetComponentsInChildren<MeshFilter>())
                {
                    Mesh mesh = filter.sharedMesh;
                    if (mesh == null)
                    {
                        continue;
                    }
                    MeshRenderer renderer = filter.GetComponent<MeshRenderer>();
                    Debug.Log(
                        $"{filter.name}: emitted mesh.bounds={mesh.bounds} "
                        + $"renderer.bounds={(renderer != null ? renderer.bounds.ToString() : "<none>")}",
                        filter);
                    mesh.RecalculateBounds();
                    Debug.Log(
                        $"{filter.name}: after RecalculateBounds mesh.bounds={mesh.bounds} "
                        + $"renderer.bounds={(renderer != null ? renderer.bounds.ToString() : "<none>")}",
                        filter);
                }
            }
        }

        [MenuItem(MenuPath)]
        private static void Toggle()
        {
            enabled = !enabled;
            Menu.SetChecked(MenuPath, enabled);
            SceneView.RepaintAll();
        }

        private static void OnSceneGui(SceneView view)
        {
            if (!enabled)
            {
                return;
            }
            foreach (Transform selection in Selection.transforms)
            {
                DrawPivotAxes(selection);
                Bounds combined = new Bounds(selection.position, Vector3.zero);
                bool hasRenderer = false;
                foreach (MeshFilter filter in selection.GetComponentsInChildren<MeshFilter>())
                {
                    Mesh mesh = filter.sharedMesh;
                    if (mesh == null)
                    {
                        continue;
                    }
                    Transform node = filter.transform;
                    DrawOrientedBox(node.localToWorldMatrix, mesh.bounds, MeshBoundsColor);
                    MeshRenderer renderer = filter.GetComponent<MeshRenderer>();
                    if (renderer == null)
                    {
                        continue;
                    }
                    Handles.color = RendererBoundsColor;
                    Handles.DrawWireCube(renderer.bounds.center, renderer.bounds.size);
                    combined.Encapsulate(renderer.bounds);
                    hasRenderer = true;
                }
                if (hasRenderer)
                {
                    Handles.color = CenterLinkColor;
                    Handles.DrawLine(selection.position, combined.center);
                    Handles.DrawWireCube(combined.center, combined.size * 0.02f);
                }
            }
        }

        private static void DrawPivotAxes(Transform node)
        {
            Handles.color = Color.red;
            Handles.DrawLine(node.position, node.position + node.right * 0.01f);
            Handles.color = Color.green;
            Handles.DrawLine(node.position, node.position + node.up * 0.01f);
            Handles.color = Color.blue;
            Handles.DrawLine(node.position, node.position + node.forward * 0.01f);
        }

        private static void DrawOrientedBox(Matrix4x4 matrix, Bounds bounds, Color color)
        {
            Handles.color = color;
            Vector3 min = bounds.min;
            Vector3 max = bounds.max;
            Vector3[] corners =
            {
                matrix.MultiplyPoint3x4(new Vector3(min.x, min.y, min.z)),
                matrix.MultiplyPoint3x4(new Vector3(max.x, min.y, min.z)),
                matrix.MultiplyPoint3x4(new Vector3(min.x, max.y, min.z)),
                matrix.MultiplyPoint3x4(new Vector3(max.x, max.y, min.z)),
                matrix.MultiplyPoint3x4(new Vector3(min.x, min.y, max.z)),
                matrix.MultiplyPoint3x4(new Vector3(max.x, min.y, max.z)),
                matrix.MultiplyPoint3x4(new Vector3(min.x, max.y, max.z)),
                matrix.MultiplyPoint3x4(new Vector3(max.x, max.y, max.z)),
            };
            int[][] edges =
            {
                new[] { 0, 1 }, new[] { 0, 2 }, new[] { 1, 3 }, new[] { 2, 3 },
                new[] { 4, 5 }, new[] { 4, 6 }, new[] { 5, 7 }, new[] { 6, 7 },
                new[] { 0, 4 }, new[] { 1, 5 }, new[] { 2, 6 }, new[] { 3, 7 },
            };
            foreach (int[] edge in edges)
            {
                Handles.DrawLine(corners[edge[0]], corners[edge[1]]);
            }
        }
    }
}
