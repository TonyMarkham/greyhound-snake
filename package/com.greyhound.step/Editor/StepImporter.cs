using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

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

        // Fallback density when the STEP file carries no material density
        // (g/cm3); the file's own value wins when present.
        private const double DefaultDensityGPerCm3 = 1.0;

        private const uint NoIndex = uint.MaxValue;

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
            string assetName = Path.GetFileNameWithoutExtension(ctx.assetPath);
            StepHost host = StepHost.GetShared();
            using (StepDocument doc = host.OpenStep(ctx.assetPath))
            {
                HostSceneCounts sceneCounts = doc.SceneCounts(Deflection, AngleRadians, Scale);

                uint[] nodes = new uint[sceneCounts.NodeCount * 4];
                float[] transforms = new float[sceneCounts.NodeCount * 12];
                byte[] names = new byte[sceneCounts.NameBytes];
                doc.SceneFill(nodes, transforms, names);

                float[] colors = new float[sceneCounts.ColorCount * 4];
                doc.ColorFill(colors);

                // One Unity Mesh asset per unique mesh; repeated instances
                // reference the same asset.
                Mesh[] meshes = new Mesh[sceneCounts.MeshCount];
                uint[][] meshSubmeshColors = new uint[sceneCounts.MeshCount][];
                var meshProperties = new System.Collections.Generic.List<HostMeshProperties>();
                for (uint meshIndex = 0; meshIndex < sceneCounts.MeshCount; meshIndex++)
                {
                    HostMeshCounts counts = doc.MeshCounts(meshIndex);
                    UnityVertex[] vertices = new UnityVertex[counts.VertexCount];
                    uint[] indices = new uint[counts.IndexCount];
                    UnitySubMesh[] submeshes = new UnitySubMesh[counts.SubmeshCount];
                    uint[] submeshColors = new uint[counts.SubmeshCount];
                    doc.MeshFill(meshIndex, vertices, indices, submeshes, submeshColors);

                    Mesh mesh = new Mesh
                    {
                        name = $"{assetName}_mesh{meshIndex}",
                    };
                    // Advanced data-oriented API order (unity-mesh.md):
                    // vertex buffer, index buffer, submeshes, bounds, upload.
                    // Bounds are supplied by the host, so recalculation is
                    // skipped.
                    mesh.SetVertexBufferParams((int)counts.VertexCount, VertexLayout);
                    mesh.SetVertexBufferData(vertices, 0, 0, (int)counts.VertexCount);
                    mesh.SetIndexBufferParams((int)counts.IndexCount, IndexFormat.UInt32);
                    mesh.SetIndexBufferData(indices, 0, 0, (int)counts.IndexCount);
                    mesh.SetSubMeshes(BuildSubMeshes(submeshes), MeshUpdateFlags.DontRecalculateBounds);
                    mesh.bounds = counts.Bounds.ToBounds();
                    mesh.UploadMeshData(false);

                    meshes[meshIndex] = mesh;
                    meshSubmeshColors[meshIndex] = submeshColors;
                    meshProperties.Add(doc.MeshProperties(meshIndex));
                    ctx.AddObjectToAsset($"mesh{meshIndex}", mesh);
                }

                Material[] palette = BuildPalette(assetName, colors);
                for (int i = 0; i < palette.Length; i++)
                {
                    ctx.AddObjectToAsset($"material{i}", palette[i]);
                }

                // Depth-first pre-order from the native walk: every parent
                // precedes its children, so a plain array of transforms
                // builds the hierarchy in one pass.
                string[] nodeNames = ReadNames(names, nodes);
                Transform[] nodeTransforms = new Transform[sceneCounts.NodeCount];
                GameObject root = null;
                for (uint i = 0; i < sceneCounts.NodeCount; i++)
                {
                    uint parent = nodes[i * 4];
                    GameObject node = new GameObject(nodeNames[i]);
                    if (parent == NoIndex)
                    {
                        if (root != null)
                        {
                            throw new InvalidOperationException("multiple scene roots");
                        }
                        root = node;
                    }
                    else
                    {
                        if (parent >= i)
                        {
                            throw new InvalidOperationException($"node {i} parent {parent} is not a predecessor");
                        }
                        node.transform.SetParent(nodeTransforms[parent], false);
                    }
                    ApplyTransform(node.transform, transforms, i);
                    uint meshIndex = nodes[i * 4 + 1];
                    if (meshIndex != NoIndex)
                    {
                        node.AddComponent<MeshFilter>().sharedMesh = meshes[meshIndex];
                        node.AddComponent<MeshRenderer>().sharedMaterials =
                            SubmeshMaterials(palette, meshSubmeshColors[meshIndex]);
                        StepMassProperties.Create(node.transform, meshProperties[(int)meshIndex], DefaultDensityGPerCm3);
                        if (node.TryGetComponent<Rigidbody>(out Rigidbody body))
                        {
                            node.GetComponent<StepMassProperties>().ApplyTo(body);
                        }
                    }
                    nodeTransforms[i] = node.transform;
                }

                if (root == null)
                {
                    throw new InvalidOperationException("the scene has no root node");
                }
                root.name = assetName;
                ctx.AddObjectToAsset("root", root);
                ctx.SetMainObject(root);
            }
        }

        private static string[] ReadNames(byte[] names, uint[] nodes)
        {
            var result = new string[nodes.Length / 4];
            for (uint i = 0; i < result.Length; i++)
            {
                uint offset = nodes[i * 4 + 2];
                uint length = nodes[i * 4 + 3];
                result[i] = Encoding.UTF8.GetString(names, (int)offset, (int)length);
            }
            return result;
        }

        private static void ApplyTransform(Transform target, float[] transforms, uint nodeIndex)
        {
            float a11 = transforms[nodeIndex * 12];
            float a12 = transforms[nodeIndex * 12 + 1];
            float a13 = transforms[nodeIndex * 12 + 2];
            float a21 = transforms[nodeIndex * 12 + 4];
            float a22 = transforms[nodeIndex * 12 + 5];
            float a23 = transforms[nodeIndex * 12 + 6];
            float a31 = transforms[nodeIndex * 12 + 8];
            float a32 = transforms[nodeIndex * 12 + 9];
            float a33 = transforms[nodeIndex * 12 + 10];

            var column0 = new Vector3(a11, a21, a31);
            var column1 = new Vector3(a12, a22, a32);
            var column2 = new Vector3(a13, a23, a33);
            var scale = new Vector3(column0.magnitude, column1.magnitude, column2.magnitude);
            var rotation = Matrix4x4.identity;
            rotation.SetColumn(0, column0 / scale.x);
            rotation.SetColumn(1, column1 / scale.y);
            rotation.SetColumn(2, column2 / scale.z);
            // A negative determinant (a mirrored placement) cannot be
            // represented by a rotation; mirroring one scale axis keeps the
            // placement correct at the cost of Unity's negative-scale
            // caveats (gap G9 tracks the full solution).
            if (rotation.determinant < 0.0f)
            {
                scale.x = -scale.x;
            }
            target.localPosition = new Vector3(
                transforms[nodeIndex * 12 + 3],
                transforms[nodeIndex * 12 + 7],
                transforms[nodeIndex * 12 + 11]);
            target.localRotation = rotation.rotation;
            target.localScale = scale;
        }

        private static Material[] SubmeshMaterials(Material[] palette, uint[] submeshColors)
        {
            var materials = new Material[submeshColors.Length];
            for (int i = 0; i < submeshColors.Length; i++)
            {
                if (submeshColors[i] >= palette.Length)
                {
                    throw new InvalidOperationException(
                        $"submesh {i} references color {submeshColors[i]} of {palette.Length}");
                }
                materials[i] = palette[submeshColors[i]];
            }
            return materials;
        }

        private static Material[] BuildPalette(string assetName, float[] colors)
        {
            Shader shader = Shader.Find(UrpLitShaderName)
                ?? Shader.Find("Standard")
                ?? throw new InvalidOperationException("no Lit shader available");

            // STEP colors decode as sRGB on the native side; a linear-color-space
            // project converts material colors on upload. One material per
            // palette entry, in palette order.
            int count = colors.Length / 4;
            var materials = new Material[count];
            for (int i = 0; i < count; i++)
            {
                var color = new Color(colors[i * 4], colors[i * 4 + 1], colors[i * 4 + 2], colors[i * 4 + 3]);
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
