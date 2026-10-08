using System.Runtime.InteropServices;

namespace Greyhound.Step
{
    [StructLayout(LayoutKind.Sequential)]
    public struct HostMeshCounts
    {
        public uint VertexCount;
        public uint IndexCount;
        public uint SubmeshCount;
        public UnityBounds Bounds;
    }
}