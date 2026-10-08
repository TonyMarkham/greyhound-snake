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
            // Unity's Bounds constructor takes (center, size); passing Min
            // as the center anchored every imported bounds box at its own
            // min corner (found with the bounds gizmo: extents matched but
            // the center sat at Min).
            return new Bounds((Min + Max) * 0.5f, Max - Min);
        }
    }
}
