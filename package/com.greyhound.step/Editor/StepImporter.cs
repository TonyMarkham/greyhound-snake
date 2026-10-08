using System;
using System.IO;

using UnityEngine;
using UnityEngine.Rendering;

using UnityEditor.AssetImporters;

namespace Greyhound.Step
{
    [ScriptedImporter(version: 1, ext: "stp")]
    internal sealed class StepImporter : ScriptedImporter
    {
        private const double Deflection = 0.01;
        private const double AngleRadians = 0.5;
        private const double Scale = 0.001;

        private const string UrpLitShaderName = "Universal Render Pipeline/Lit";

        private static readonly VertexAttributeDescriptor[] VertexLayout =
        {
            new VertexAttributeDescriptor(VertexAttribute.Position, VertexAttributeFormat.Float32, 3, 0),
            new VertexAttributeDescriptor(VertexAttribute.Normal, VertexAttributeFormat.Float32, 3, 0),
        };

        public override void OnImportAsset(AssetImportContext ctx)
        {
            try
            {
                Import(ctx);
            }
            catch (Exception exception)
            {
                ctx.LogImportError($"{ctx.assetPath}: {exception.Message}");
            }
        }

        private static void Import(AssetImportContext ctx)
        {
            StepHost host = StepHost.GetShared();
            using (StepDocument doc = host.OpenStep(ctx.assetPath))
            {
                HostMeshCounts counts = doc.MeshCounts(Deflection, AngleRadians, Scale);

                UnityVertex[] vertices = new UnityVertex[counts.VertexCount];
                uint[] indices = new uint[counts.IndexCount];
                UnitySubMesh[] submeshes = new UnitySubMesh[counts.SubmeshCount];
                uint[] submeshColors = new uint[counts.SubmeshCount];
                float[] colors = new float[counts.ColorCount * 4];
                doc.MeshFill(vertices, indices, submeshes, submeshColors, colors);

                Mesh mesh = new Mesh
                {
                    name = Path.GetFileNameWithoutExtension(ctx.assetPath),
                };
                // Advanced data-oriented API order (unity-mesh.md): vertex
                // buffer, index buffer, submeshes, bounds, upload. Bounds
                // are supplied by the host, so recalculation is skipped.
                mesh.SetVertexBufferParams((int)counts.VertexCount, VertexLayout);
                mesh.SetVertexBufferData(vertices, 0, 0, (int)counts.VertexCount);
                mesh.SetIndexBufferParams((int)counts.IndexCount, IndexFormat.UInt32);
                mesh.SetIndexBufferData(indices, 0, 0, (int)counts.IndexCount);
                mesh.SetSubMeshes(BuildSubMeshes(submeshes), MeshUpdateFlags.DontRecalculateBounds);
                mesh.bounds = counts.Bounds.ToBounds();
                mesh.UploadMeshData(false);

                Material[] materials = BuildMaterials(mesh.name, submeshColors, colors);
                for (int i = 0; i < materials.Length; i++)
                {
                    ctx.AddObjectToAsset($"material{i}", materials[i]);
                }
                ctx.AddObjectToAsset("mesh", mesh);

                // A bare Mesh cannot hold materials; the imported object is a
                // GameObject root so the renderer carries the submesh colors.
                GameObject root = new GameObject(mesh.name);
                root.AddComponent<MeshFilter>().sharedMesh = mesh;
                root.AddComponent<MeshRenderer>().sharedMaterials = materials;
                ctx.AddObjectToAsset("root", root);
                ctx.SetMainObject(root);
            }
        }

        private static Material[] BuildMaterials(string assetName, uint[] submeshColors, float[] colors)
        {
            // STEP colors decode as sRGB on the native side; a linear-color-space
            // project converts material colors on upload. One material per distinct
            // submesh color, in first-appearance order.
            var distinct = new System.Collections.Generic.List<uint>();
            foreach (uint colorIndex in submeshColors)
            {
                if (!distinct.Contains(colorIndex))
                {
                    distinct.Add(colorIndex);
                }
            }

            Shader shader = Shader.Find(UrpLitShaderName)
                ?? Shader.Find("Standard")
                ?? throw new InvalidOperationException("no Lit shader available");

            var materials = new Material[distinct.Count];
            for (int i = 0; i < distinct.Count; i++)
            {
                uint colorIndex = distinct[i];
                Color color = Color.white;
                if ((int)colorIndex * 4 + 3 < colors.Length)
                {
                    color = new Color(
                        colors[colorIndex * 4],
                        colors[colorIndex * 4 + 1],
                        colors[colorIndex * 4 + 2],
                        colors[colorIndex * 4 + 3]);
                }
                materials[i] = new Material(shader)
                {
                    name = $"{assetName}_color{i}",
                    hideFlags = HideFlags.HideInHierarchy,
                };
                if (materials[i].HasProperty("_BaseColor"))
                {
                    materials[i].SetColor("_BaseColor", color);
                }
                else
                {
                    materials[i].SetColor("_Color", color);
                }
            }
            return materials;
        }

        private static SubMeshDescriptor[] BuildSubMeshes(UnitySubMesh[] submeshes)
        {
            var descriptors = new SubMeshDescriptor[submeshes.Length];
            for (int i = 0; i < submeshes.Length; i++)
            {
                descriptors[i] = new SubMeshDescriptor
                {
                    topology = MeshTopology.Triangles,
                    indexStart = (int)submeshes[i].IndexStart,
                    indexCount = (int)submeshes[i].IndexCount,
                    firstVertex = (int)submeshes[i].FirstVertex,
                    vertexCount = (int)submeshes[i].VertexCount,
                    baseVertex = 0,
                };
            }
            return descriptors;
        }
    }
}