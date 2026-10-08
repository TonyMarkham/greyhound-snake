using System.Runtime.InteropServices;

using UnityEngine;

namespace Greyhound.Step
{
    [StructLayout(LayoutKind.Sequential)]
    public struct UnityBounds
    {
        public Vector3 Min;
        public Vector3 Max;

        public Bounds ToBounds()
        {
            return new Bounds(Min, Max - Min);
        }
    }
}