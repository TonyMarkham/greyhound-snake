using System.Runtime.InteropServices;

namespace Greyhound.Step
{
    // Mirrors the host's repr(C) mesh-properties struct: volume and file
    // density (0.0 when the file carries none), the body-local centre of
    // gravity in meters, the density-free gyration radii in mm, and the
    // three principal axes of inertia as body-local unit direction vectors
    // (row-major: rows 0..2 = first..third axis).
    [StructLayout(LayoutKind.Sequential)]
    public struct HostMeshProperties
    {
        public float VolumeMm3;
        public float FileDensity;
        public float CentreOfGravityX;
        public float CentreOfGravityY;
        public float CentreOfGravityZ;
        public float GyrationRadius1;
        public float GyrationRadius2;
        public float GyrationRadius3;
        public float Axis1X;
        public float Axis1Y;
        public float Axis1Z;
        public float Axis2X;
        public float Axis2Y;
        public float Axis2Z;
        public float Axis3X;
        public float Axis3Y;
        public float Axis3Z;
    }
}
