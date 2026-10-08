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

        private static readonly VertexAttributeDescriptor[] VertexLayout =
        {
            new VertexAttributeDescriptor(VertexAttribute.Position, VertexAttributeFormat.Float32, 3, 0),
            new VertexAttributeDescriptor(VertexAttribute.Normal, VertexAttributeFormat.Float32, 3, 0),
        };

        public override void OnImportAsset(AssetImportContext ctx)
        {
            Mesh mesh;
            try
            {
                mesh = Import(ctx.assetPath);
            }
            catch (Exception exception)
            {
                ctx.LogImportError($"{ctx.assetPath}: {exception.Message}");
                return;
            }
            ctx.AddObjectToAsset("mesh", mesh);
            ctx.SetMainObject(mesh);
        }

        private static Mesh Import(string assetPath)
        {
            StepHost host = StepHost.GetShared();
            using (StepDocument doc = host.OpenStep(assetPath))
            {
                HostMeshCounts counts = doc.MeshCounts(Deflection, AngleRadians, Scale);

                UnityVertex[] vertices = new UnityVertex[counts.VertexCount];
                uint[] indices = new uint[counts.IndexCount];
                UnitySubMesh[] submeshes = new UnitySubMesh[counts.SubmeshCount];
                doc.MeshFill(vertices, indices, submeshes);

                Mesh mesh = new Mesh
                {
                    name = Path.GetFileNameWithoutExtension(assetPath),
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
                return mesh;
            }
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