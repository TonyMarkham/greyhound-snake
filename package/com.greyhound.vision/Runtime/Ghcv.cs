using System;
using System.IO;
using System.Runtime.InteropServices;

using Unity.Scripting.LifecycleManagement;

namespace Greyhound.Vision
{
    // P/Invoke surface of the ghcv shim - the OpenCV ArUco detector
    // behind a flat C ABI (crates/opencv-sys). Encoded image bytes in,
    // DICT_4X4_1000 tag ids out; the shim is stateless, so every call
    // is a pure function and no OpenCV type crosses the boundary.
    //
    // Loading. Unity resolves "ghcv" against the package's plugin
    // import (Runtime/Plugins/shim/<arch>/ghcv.dll, <arch> the
    // platform's plugin architecture). The shim's OpenCV runtime
    // dependencies (opencv_*500.dll) are not Unity plugins - they are
    // the payload the package's provisioner stages into
    // Runtime/Plugins/opencv/<arch>/lib - so they are LoadLibrary'd by
    // absolute path from that directory before the first call: the
    // loader satisfies an import from an already-loaded module of the
    // same name, so no PATH mutation and no process-wide search-path
    // change. In the editor the provisioner's InitializeOnLoad kickoff
    // runs at editor assembly load, before any detection; a load
    // failure surfaces here as the first call's error message - on a
    // platform whose payload entry has not landed the preload no-ops
    // and the message says so.
    //
    // NoAutoStaticsCleanup: the flags below record the process's
    // native module load state — OpenCV modules and the shim, once
    // mapped, are never unloaded by Unity's scene unloads or domain
    // reloads, so resetting them on reload would only re-run identical
    // loads. The attribute silences Unity 6000.7's lifecycle analyzer
    // (UAL0010/UAL0013), which demands the choice on every static type
    // holding static state; the analyzer requires the partial
    // declaration alongside it.
    [NoAutoStaticsCleanup]
    public static partial class Ghcv
    {
        // The ABI version this surface speaks; the shim reports the
        // same value from ghcv_abi_version. Bump both on any signature
        // change (the shim's doc comment pins the same rule).
        private const int RequiredAbiVersion = 1;

        // Detection buffer capacity: the capture frames hold at most
        // four markers; 256 is generous headroom for any other image.
        private const int MaxIds = 256;

        // The module name Unity resolves against the plugin import;
        // the shim's exports carry this prefix (ghcv_abi_version,
        // ghcv_aruco_detect).
        private const string Library = "ghcv";

        private static string PluginsDirectory()
        {
            // The same working-directory-relative resolution the step
            // package's NativeMethods uses for its plugins directory:
            // in the editor the working directory is the project root,
            // and file:/embedded/registry packages alike surface under
            // Packages/<name>.
            return Path.GetFullPath("Packages/com.greyhound.vision/Runtime/Plugins");
        }

        // The payload directory holding the OpenCV runtime libraries
        // the shim's import table names - the layout the fetch pins'
        // dest defines.
        public static string OpenCVLibraryDirectory()
        {
            return Path.Combine(
                PluginsDirectory(), "opencv",
                Greyhound.Step.NativeMethods.PluginArch(), "lib");
        }

        private static bool opencvPreloaded;
        private static bool shimProbed;

        [DllImport(Library, EntryPoint = "ghcv_abi_version", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
        private static extern int AbiVersion();

        [DllImport(Library, EntryPoint = "ghcv_aruco_detect", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
        private static extern int ArucoDetect(byte[] data, ulong len, int[] idsOut, ulong maxIds);

        // Preloads the OpenCV runtime DLLs from one directory: best
        // effort per DLL, because a DLL whose own dependencies are
        // missing must not fail the ones the shim actually needs.
        // Windows-only by the loader it names; on every other platform
        // this no-ops (the payload entries for linux/macos land with
        // their shim support). Idempotent; call before the first
        // Detect.
        public static void Preload(string openCVBinDirectory)
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                || opencvPreloaded
                || string.IsNullOrEmpty(openCVBinDirectory)
                || !Directory.Exists(openCVBinDirectory))
            {
                return;
            }
            foreach (string dll in Directory.GetFiles(openCVBinDirectory, "opencv_*.dll"))
            {
                LoadLibraryEx(dll, IntPtr.Zero, LoadWithAlteredSearchPath);
            }
            opencvPreloaded = true;
        }

        // Detects DICT_4X4_1000 markers in one encoded image (PNG,
        // JPEG, ... - anything cv::imdecode reads); the ids in
        // detection order. A shim error (negative count) or a load
        // failure surfaces as an empty list with a message, so one bad
        // image cannot end the caller's pass.
        public static int[] Detect(byte[] image, out string error)
        {
            error = null;
            if (!EnsureLoaded(out error))
            {
                return Array.Empty<int>();
            }
            var ids = new int[MaxIds];
            int count = ArucoDetect(image, (ulong)image.Length, ids, (ulong)MaxIds);
            if (count < 0)
            {
                error = $"shim error code {count}";
                return Array.Empty<int>();
            }
            return ids[..count];
        }

        // True when the shim is loaded and reports the expected ABI.
        // The first Detect preloads the payload directory and probes
        // the shim - the first P/Invoke proves the whole load chain,
        // OpenCV dependencies included - and the probe result is
        // cached for the process: the shim and its dependencies are
        // never unloaded or reloaded.
        private static bool EnsureLoaded(out string error)
        {
            error = null;
            if (shimProbed)
            {
                return true;
            }
            if (!opencvPreloaded)
            {
                Preload(OpenCVLibraryDirectory());
            }
            try
            {
                int reported = AbiVersion();
                if (reported != RequiredAbiVersion)
                {
                    error = $"abi mismatch: shim {reported}, expected {RequiredAbiVersion}";
                    return false;
                }
            }
            catch (DllNotFoundException exception)
            {
                error = "the ghcv shim or its OpenCV dependencies could not load: "
                    + exception.Message;
                return false;
            }
            shimProbed = true;
            return true;
        }

        private const int LoadWithAlteredSearchPath = 0x8;

        // kernel32 is named unconditionally (a DllImport compiles as
        // metadata everywhere) but is only ever called behind the
        // IsOSPlatform(Windows) gate in Preload.
        [DllImport("kernel32", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr LoadLibraryEx(string fileName, IntPtr reserved, int flags);
    }
}
