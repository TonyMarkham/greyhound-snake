using System.Runtime.InteropServices;

namespace Greyhound.Step
{
    [StructLayout(LayoutKind.Sequential)]
    public struct UnitySubMesh
    {
        public uint IndexStart;
        public uint IndexCount;
        public uint FirstVertex;
        public uint VertexCount;
    }
}