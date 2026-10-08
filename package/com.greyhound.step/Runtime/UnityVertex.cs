using System.Runtime.InteropServices;

using UnityEngine;

namespace Greyhound.Step
{
    [StructLayout(LayoutKind.Sequential)]
    public struct UnityVertex
    {
        public Vector3 Position;
        public Vector3 Normal;
    }
}