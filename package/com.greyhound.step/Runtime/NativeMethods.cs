using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Greyhound.Step
{
    // P/Invoke surface of the importer-host cdylib. Unity resolves
    // "importer_host" against the package's Runtime/Plugins/x86_64 plugin.
    public static class NativeMethods
    {
        private const string Library = "importer_host";

        public const int AbiVersion = 4;

        public static string OcctLibraryDirectory()
        {
            return Path.Combine(PluginsDirectory(), "occt", "x86_64", "lib");
        }

        public static string ShimPath()
        {
            return Path.Combine(PluginsDirectory(), "shim", "x86_64", "libgreyhound_occt.so");
        }

        private static string PluginsDirectory()
        {
            return Path.GetFullPath("Packages/com.greyhound.step/Runtime/Plugins");
        }

        [DllImport(Library, EntryPoint = "greyhound_host_version", ExactSpelling = true)]
        public static extern uint HostVersion();

        [DllImport(Library, EntryPoint = "greyhound_host_last_error", ExactSpelling = true)]
        public static extern IntPtr LastError();

        [DllImport(Library, EntryPoint = "greyhound_host_new", ExactSpelling = true)]
        public static extern IntPtr NewHost(string occtDir, string shimPath);

        [DllImport(Library, EntryPoint = "greyhound_host_free", ExactSpelling = true)]
        public static extern void FreeHost(IntPtr host);

        [DllImport(Library, EntryPoint = "greyhound_host_open_step", ExactSpelling = true)]
        public static extern IntPtr OpenStep(IntPtr host, string path);

        [DllImport(Library, EntryPoint = "greyhound_host_close_step", ExactSpelling = true)]
        public static extern void CloseStep(IntPtr doc);

        [DllImport(Library, EntryPoint = "greyhound_host_scene_counts", ExactSpelling = true)]
        public static extern int SceneCounts(
            IntPtr doc,
            double deflection,
            double angleRad,
            double scale,
            out HostSceneCounts counts);

        [DllImport(Library, EntryPoint = "greyhound_host_scene_fill", ExactSpelling = true)]
        public static extern int SceneFill(
            IntPtr doc,
            uint[] nodes,
            float[] transforms,
            byte[] names);

        [DllImport(Library, EntryPoint = "greyhound_host_color_fill", ExactSpelling = true)]
        public static extern int ColorFill(IntPtr doc, float[] colors);

        [DllImport(Library, EntryPoint = "greyhound_host_mesh_properties", ExactSpelling = true)]
        public static extern int MeshProperties(IntPtr doc, uint mesh, out HostMeshProperties properties);

        [DllImport(Library, EntryPoint = "greyhound_host_mesh_counts", ExactSpelling = true)]
        public static extern int MeshCounts(IntPtr doc, uint mesh, out HostMeshCounts counts);

        [DllImport(Library, EntryPoint = "greyhound_host_mesh_fill", ExactSpelling = true)]
        public static extern int MeshFill(
            IntPtr doc,
            uint mesh,
            UnityVertex[] vertices,
            uint[] indices,
            UnitySubMesh[] submeshes,
            uint[] submeshColors);

        public static string TakeLastError()
        {
            IntPtr pointer = LastError();
            return pointer == IntPtr.Zero ? "native operation failed" : Marshal.PtrToStringAnsi(pointer);
        }
    }
}
