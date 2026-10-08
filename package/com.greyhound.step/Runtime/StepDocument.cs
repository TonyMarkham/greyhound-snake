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

        public HostSceneCounts SceneCounts(double deflection, double angleRad, double scale)
        {
            var counts = new HostSceneCounts();
            int status = NativeMethods.SceneCounts(handle, deflection, angleRad, scale, out counts);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
            return counts;
        }

        // The blittable element types keep these arrays pinned for the
        // duration of the native call; no explicit GCHandle is required.
        public void SceneFill(uint[] nodes, float[] transforms, byte[] names)
        {
            int status = NativeMethods.SceneFill(handle, nodes, transforms, names);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
        }

        public void ColorFill(float[] colors)
        {
            int status = NativeMethods.ColorFill(handle, colors);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
        }

        public HostMeshCounts MeshCounts(uint mesh)
        {
            var counts = new HostMeshCounts();
            int status = NativeMethods.MeshCounts(handle, mesh, out counts);
            if (status != 0)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
            return counts;
        }

        public void MeshFill(
            uint mesh,
            UnityVertex[] vertices,
            uint[] indices,
            UnitySubMesh[] submeshes,
            uint[] submeshColors)
        {
            int status = NativeMethods.MeshFill(handle, mesh, vertices, indices, submeshes, submeshColors);
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
