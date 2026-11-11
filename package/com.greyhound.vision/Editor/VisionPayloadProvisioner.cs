using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;
using UnityEngine.Networking;

#if UNITY_6000_5_OR_NEWER
using Unity.Scripting.LifecycleManagement;
#endif

namespace Greyhound.Vision
{
    // Provisions the package's native payloads at editor load, <arch>
    // from the step package's NativeMethods.PluginArch(); the UPM
    // package ships light and downloads pinned release archives instead:
    //
    //   group           folder                            missing entry
    //   opencv          Runtime/Plugins/opencv/<arch>/lib throws
    //   greyhound_ghcv  Runtime/Plugins/shim/<arch>       skipped
    //
    // Each folder is owned through its sibling <folder>.provenance.toml
    // marker. The ghcv folder also accepts an unmarked folder that
    // already holds DLLs: that is a local cargo build, and a release
    // download never overwrites it. UPM has no install-time hook, so the
    // handshake point is editor assembly load: the kickoff runs on
    // InitializeOnLoad, before any project-side detection can run. A
    // detection that arrives while provisioning is still in flight
    // fails through Ghcv's error message - the payload directory it
    // preloads does not exist yet - never with a cryptic loader error,
    // and the caller's never-wipe rule keeps the labels safe.
    //
    // The archive, its hash, and the destination contract come from the
    // staged copy of crates/extern-fetch/config.toml; `just
    // fetch-opencv` provisions the same destination from the
    // same pins, and the sibling provenance marker keeps the two
    // provisioners idempotent against each other.
    // NoAutoStaticsCleanup: the state fields intentionally outlive any
    // scene unload — the payload is provisioned once per editor
    // process, like the shared StepHost; the analyzer requires the
    // partial declaration.
#if UNITY_6000_5_OR_NEWER
    [NoAutoStaticsCleanup]
#endif
    internal static partial class VisionPayloadProvisioner
    {
        private const string PackageAssetPath = "Packages/com.greyhound.vision";

        private static VisionProvisionState state = VisionProvisionState.NotStarted;
        private static string status = string.Empty;

        // Blocks a native detection call until the payload is usable.
        // Called on every verify: Ready is a single field read.
        public static void EnsureReady()
        {
            if (state == VisionProvisionState.NotStarted)
            {
                // InitializeOnLoad runs before the first import of a
                // domain; this is only a safety net for a verify that
                // somehow precedes it.
                Kickoff();
            }
            switch (state)
            {
                case VisionProvisionState.Ready:
                    return;
                case VisionProvisionState.Running:
                    throw new InvalidOperationException(
                        "the native payload is still provisioning "
                        + $"({status}); wait for it to finish and run the verify again");
                case VisionProvisionState.Failed:
                    throw new InvalidOperationException(
                        "the native payload failed to provision: "
                        + $"{status} — fix the cause (Unity reloads the "
                        + "package on script changes) and run the verify again");
                default:
                    throw new InvalidOperationException(
                        "the native payload has not been provisioned yet");
            }
        }

        [InitializeOnLoadMethod]
        private static void KickoffOnLoad()
        {
            Kickoff();
        }

        private static void Kickoff()
        {
            if (state != VisionProvisionState.NotStarted)
            {
                return;
            }
            try
            {
                string packageRoot = PackageRoot();
                string plugins = Path.Combine(packageRoot, "Runtime", "Plugins");
                string arch = Greyhound.Step.NativeMethods.PluginArch();
                var pending = new List<(VisionFetchConfig config, string payloadDir)>();

                // OpenCV: required, and an unmarked folder is re-provisioned.
                Plan(pending,
                    VisionFetchConfig.Load(packageRoot, "opencv", required: true),
                    Path.Combine(plugins, "opencv", arch, "lib"),
                    keepUnmarked: false);
                // Our own ghcv shim: optional (no entry for this platform =
                // no download), and a folder that already holds DLLs
                // without a marker is a local cargo build — kept.
                Plan(pending,
                    VisionFetchConfig.Load(packageRoot, "greyhound_ghcv", required: false),
                    Path.Combine(plugins, "shim", arch),
                    keepUnmarked: true);

                if (pending.Count == 0)
                {
                    // Cheap domain-reload check: every folder already
                    // certifies its pin (or is a kept local build).
                    state = VisionProvisionState.Ready;
                    return;
                }
                state = VisionProvisionState.Running;
                _ = RunAsync(pending);
            }
            catch (Exception exception)
            {
                state = VisionProvisionState.Failed;
                status = exception.Message;
                Debug.LogError(
                    "[com.greyhound.vision] native payload provisioning could not start: "
                    + exception.Message);
            }
        }


        // Adds payloadDir to pending unless it is already good:
        //   config null (optional group, no entry for this platform) -> skip
        //   marker matches the pin                                   -> skip
        //   no marker, keepUnmarked, folder holds a non-.meta file   -> skip
        //     (a local cargo build; never overwritten by a release)
        //   anything else (no marker, or a marker for another pin)   -> download
        // The deletion of the old folder only happens after the new zip
        // has passed its hash check (Install), so a failed run cannot
        // leave the payload destroyed without a path to recovery.
        private static void Plan(
            List<(VisionFetchConfig config, string payloadDir)> pending,
            VisionFetchConfig config,
            string payloadDir,
            bool keepUnmarked)
        {
            if (config == null)
            {
                return;
            }
            VisionProvenance marker = VisionProvenance.Read(MarkerPath(payloadDir));
            if (marker != null && marker.Matches(config))
            {
                return;
            }
            if (marker == null && keepUnmarked && HoldsFiles(payloadDir))
            {
                return;
            }
            pending.Add((config, payloadDir));
        }

        private static bool HoldsFiles(string dir)
        {
            if (!Directory.Exists(dir))
            {
                return false;
            }
            foreach (string file in Directory.EnumerateFiles(dir))
            {
                if (!file.EndsWith(".meta", StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static string MarkerPath(string payloadDir)
        {
            return payloadDir + ".provenance.toml";
        }

        // Provisions each pending folder in order; the first failure
        // stops the run and is reported through Fail. One Refresh at the
        // end imports everything that landed — including a downloaded
        // ghcv DLL, which Unity must import as a plugin before
        // [DllImport("ghcv")] can resolve it.
        private static async Task RunAsync(
            List<(VisionFetchConfig config, string payloadDir)> pending)
        {
            try
            {
                foreach ((VisionFetchConfig config, string payloadDir) in pending)
                {
                    status = $"downloading {config.Url}";
                    await ProvisionAsync(config, payloadDir);
                }
                AssetDatabase.Refresh();
                state = VisionProvisionState.Ready;
            }
            catch (Exception exception)
            {
                Fail(exception);
            }
        }

        private static async Task ProvisionAsync(VisionFetchConfig config, string payloadDir)
        {
            if (config.DownloadLocation.Length == 0)
            {
                // Memory mode (the tracked default): the whole archive
                // lands in a managed byte array (DownloadHandlerBuffer),
                // never on disk — the behavior for machines whose IT
                // policy locks down temp directories, and there is no
                // cleanup step. Cost is a transient managed allocation
                // of the archive size — tens of MB, once per release.
                using (var request = new UnityWebRequest(config.Url, UnityWebRequest.kHttpVerbGET)
                {
                    downloadHandler = new DownloadHandlerBuffer(),
                })
                {
                    await AwaitDownload(request.SendWebRequest());
                    ThrowIfFailed(request, config);
                    byte[] zipBytes = request.downloadHandler.data;
                    VerifyHash(config, Sha256Hex(zipBytes));
                    int files = Extract(zipBytes, StagingDir(payloadDir));
                    Install(config, payloadDir, files);
                }
                return;
            }

            // File mode (per-machine override): stage the zip inside the
            // configured directory. The directory must already exist —
            // the provisioner never creates directories on a machine
            // whose policies govern exactly which paths are approved.
            if (!Directory.Exists(config.DownloadLocation))
            {
                throw new InvalidOperationException(
                    $"download_location {config.DownloadLocation} does not exist; "
                    + "the provisioner never creates directories");
            }
            // One zip name per group ("opencv-payload.zip",
            // "greyhound_ghcv-payload.zip"): the runs are sequential, but
            // a distinct name keeps a leftover from one group from ever
            // being mistaken for another's.
            string zipPath = Path.Combine(config.DownloadLocation, $"{config.Group}-payload.zip");
            try
            {
                using (var request = new UnityWebRequest(config.Url, UnityWebRequest.kHttpVerbGET)
                {
                    downloadHandler = new DownloadHandlerFile(zipPath) { removeFileOnAbort = true },
                })
                {
                    await AwaitDownload(request.SendWebRequest());
                    ThrowIfFailed(request, config);
                }
                VerifyHash(config, Sha256Hex(zipPath));
                int files = Extract(zipPath, StagingDir(payloadDir));
                Install(config, payloadDir, files);
            }
            finally
            {
                // The staged zip is disposable; failing to delete it
                // must not fail the provisioning.
                TryDelete(zipPath);
            }
        }

        private static void ThrowIfFailed(UnityWebRequest request, VisionFetchConfig config)
        {
            if (request.result != UnityWebRequest.Result.Success)
            {
                throw new InvalidOperationException(
                    $"download of {config.Url} failed "
                    + $"({request.responseCode}): {request.error}");
            }
        }

        // Task.Yield resumes on Unity's synchronization context, so the
        // continuation runs on the main thread.
        private static async Task AwaitDownload(UnityWebRequestAsyncOperation operation)
        {
            while (!operation.isDone)
            {
                await Task.Yield();
            }
        }

        private static void VerifyHash(VisionFetchConfig config, string actual)
        {
            if (!string.Equals(actual, config.Sha256, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException(
                    $"the downloaded {config.Group} payload hash " + actual
                    + " does not match the pinned sha256 " + config.Sha256
                    + "; nothing was extracted");
            }
        }

        // Extraction targets a staging directory beside the payload and
        // the result is moved into place (same volume), so a reader
        // never sees a half-populated lib/ and the move never crosses a
        // filesystem boundary. Pre-cleaned: a leftover staging tree
        // from a killed run must not mix into this one.
        private static string StagingDir(string payloadDir)
        {
            string stagingDir = StagingDirName(payloadDir);
            if (Directory.Exists(stagingDir))
            {
                Directory.Delete(stagingDir, recursive: true);
            }
            return stagingDir;
        }

        private static void Install(VisionFetchConfig config, string payloadDir, int files)
        {
            if (Directory.Exists(payloadDir))
            {
                Directory.Delete(payloadDir, recursive: true);
            }
            Directory.Move(StagingDirName(payloadDir), payloadDir);

            VisionProvenance.ForConfig(config).Write(MarkerPath(payloadDir));
            Debug.Log(
                $"[com.greyhound.vision] {config.Group} payload provisioned: "
                + $"{files} files into {payloadDir}");
        }

        private static string StagingDirName(string payloadDir)
        {
            return payloadDir + ".staging";
        }

        private static void Fail(Exception exception)
        {
            state = VisionProvisionState.Failed;
            status = exception.Message;
            Debug.LogError(
                "[com.greyhound.vision] native payload provisioning failed: "
                + exception.Message);
        }

        private static void TryDelete(string path)
        {
            if (!File.Exists(path))
            {
                return;
            }
            try
            {
                File.Delete(path);
            }
            catch (IOException)
            {
                // Disposable by design; the next run overwrites it.
            }
        }

        private static string Sha256Hex(byte[] bytes)
        {
            // One pass over the in-memory archive; lowercase hex, the
            // form the release pages and the pins use.
            using (var sha = SHA256.Create())
            {
                byte[] digest = sha.ComputeHash(bytes);
                var hex = new StringBuilder(64);
                foreach (byte b in digest)
                {
                    hex.Append(b.ToString("x2"));
                }
                return hex.ToString();
            }
        }

        private static string Sha256Hex(string path)
        {
            // Streaming: the staged archive is tens to hundreds of MB;
            // hash it without buffering it whole a second time.
            using (var sha = SHA256.Create())
            using (FileStream stream = File.OpenRead(path))
            {
                byte[] digest = sha.ComputeHash(stream);
                var hex = new StringBuilder(64);
                foreach (byte b in digest)
                {
                    hex.Append(b.ToString("x2"));
                }
                return hex.ToString();
            }
        }

        // Extracts the verified archive held in memory into dest,
        // mirroring crates/extern-fetch's archive.rs rules: refuse any
        // entry that escapes the extraction root (zip-slip), return the
        // count of files written.
        private static int Extract(byte[] zipBytes, string dest)
        {
            Directory.CreateDirectory(dest);
            using (var zipStream = new MemoryStream(zipBytes, writable: false))
            using (var archive = new ZipArchive(
                zipStream, ZipArchiveMode.Read, leaveOpen: false))
            {
                return ExtractEntries(archive, dest);
            }
        }

        private static int Extract(string zipPath, string dest)
        {
            Directory.CreateDirectory(dest);
            using (var archive = new ZipArchive(
                File.OpenRead(zipPath), ZipArchiveMode.Read, leaveOpen: false))
            {
                return ExtractEntries(archive, dest);
            }
        }

        private static int ExtractEntries(ZipArchive archive, string dest)
        {
            int files = 0;
            foreach (ZipArchiveEntry entry in archive.Entries)
            {
                string relative = EnclosedName(entry.FullName);
                string target = Path.Combine(dest, relative);
                if (relative.EndsWith("/", StringComparison.Ordinal))
                {
                    Directory.CreateDirectory(target);
                    continue;
                }
                string parent = Path.GetDirectoryName(target);
                if (!string.IsNullOrEmpty(parent))
                {
                    Directory.CreateDirectory(parent);
                }
                using (Stream source = entry.Open())
                using (FileStream output = File.Create(target))
                {
                    source.CopyTo(output);
                }
                files++;
            }
            return files;
        }

        // The zip-slip guard, mirrored from archive.rs: None never
        // happens here — a bad name throws, because a silently skipped
        // entry would pass the hash check yet deliver an incomplete
        // payload.
        private static string EnclosedName(string fullName)
        {
            string normalized = fullName.Replace('\\', '/');
            if (string.IsNullOrEmpty(normalized))
            {
                throw new InvalidOperationException("the zip contains an empty entry name");
            }
            if (Path.IsPathRooted(normalized) || normalized.StartsWith("/")
                || normalized.Contains(':'))
            {
                throw new InvalidOperationException(
                    $"zip entry {fullName} escapes the extraction root");
            }
            foreach (string segment in normalized.Split('/'))
            {
                if (segment == "..")
                {
                    throw new InvalidOperationException(
                        $"zip entry {fullName} escapes the extraction root");
                }
            }
            return normalized;
        }

        private static string PackageRoot()
        {
            // FindForAssetPath resolves file:, embedded, and registry
            // packages alike — never hardcode a checkout-relative path.
            // Fully qualified: UnityEditor.PackageInfo also exists, so
            // the bare name is ambiguous when UnityEditor is imported.
            UnityEditor.PackageManager.PackageInfo info =
                UnityEditor.PackageManager.PackageInfo.FindForAssetPath(PackageAssetPath);
            if (info == null || string.IsNullOrEmpty(info.resolvedPath))
            {
                throw new InvalidOperationException(
                    $"cannot resolve the on-disk location of {PackageAssetPath}");
            }
            return info.resolvedPath;
        }
    }
}
