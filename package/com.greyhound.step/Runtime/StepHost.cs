using System;

using Unity.Scripting.LifecycleManagement;

namespace Greyhound.Step
{
    // Process-lifetime wrapper around the native host. The host is created
    // once per editor domain and intentionally never freed: it holds the
    // dlopen handles for OCCT and the shim for the life of the process.
    // NoAutoStaticsCleanup keeps Unity's statics lifecycle analyzer happy;
    // the generator requires the partial declaration.
    [NoAutoStaticsCleanup]
    public sealed partial class StepHost
    {
        private static StepHost shared;

        private readonly IntPtr handle;

        private StepHost(IntPtr handle)
        {
            this.handle = handle;
        }

        public static StepHost GetShared()
        {
            if (shared != null)
            {
                return shared;
            }
            shared = Create();
            return shared;
        }

        private static StepHost Create()
        {
            uint version = NativeMethods.HostVersion();
            if (version != NativeMethods.AbiVersion)
            {
                throw new InvalidOperationException(
                    $"importer_host ABI version {version}, expected {NativeMethods.AbiVersion}");
            }

            IntPtr handle = NativeMethods.NewHost(
                NativeMethods.OcctLibraryDirectory(),
                NativeMethods.ShimPath());
            if (handle == IntPtr.Zero)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
            return new StepHost(handle);
        }

        public StepDocument OpenStep(string path)
        {
            IntPtr doc = NativeMethods.OpenStep(handle, path);
            if (doc == IntPtr.Zero)
            {
                throw new InvalidOperationException(NativeMethods.TakeLastError());
            }
            return new StepDocument(doc);
        }
    }
}