using System;

namespace Greyhound.Step
{
    public sealed class StepDocument : IDisposable
    {
        private IntPtr handle;

        internal StepDocument(IntPtr handle)
        {
            this.handle = handle;
        }

        public HostMeshCounts MeshCounts(double deflection, double angleRad, double scale)
        {
            var counts = new HostMeshCounts();
            int status = NativeMethods.MeshCounts(handle, deflection, angleRad, scale, out counts);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
            return counts;
        }

        // The blittable element types keep these arrays pinned for the
        // duration of the native call; no explicit GCHandle is required.
        public void MeshFill(UnityVertex[] vertices, uint[] indices, UnitySubMesh[] submeshes)
        {
            int status = NativeMethods.MeshFill(handle, vertices, indices, submeshes);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
        }

        public void Dispose()
        {
            if (handle != IntPtr.Zero)
            {
                NativeMethods.CloseStep(handle);
                handle = IntPtr.Zero;
            }
        }
    }
}