using System;
using System.IO;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;

using UnityEditor;
using UnityEngine;
using UnityEngine.Networking;

using Unity.Scripting.LifecycleManagement;

namespace Greyhound.Step
{
    // Provisions the OCCT payload — the native shared libraries the
    // importer dlopen's — into the package's
    // Runtime/Plugins/occt/<arch>/lib at editor load, <arch> from
    // NativeMethods.PluginArch(): the UPM package
    // ships light and downloads the pinned
    // release archive instead. UPM has no install-time hook, so the
    // handshake point is editor assembly load; a STEP import that
    // arrives before provisioning is done fails with a clear message
    // (StepImporter calls EnsureReady), never with a cryptic loader
    // error.
    //
    // The archive, its hash, and the destination contract come from the
    // staged copy of crates/extern-fetch/config.toml; `just fetch-occt`
    // provisions the same destination from the
    // same pins, and the sibling provenance marker keeps the two
    // provisioners idempotent against each other.
    // NoAutoStaticsCleanup: the state fields intentionally outlive any
    // scene unload — the payload is provisioned once per editor
    // process, like the shared StepHost; the analyzer requires the
    // partial declaration.
    [NoAutoStaticsCleanup]
    internal static partial class StepPayloadProvisioner
    {
        private const string PackageAssetPath = "Packages/com.greyhound.step";

        private static StepProvisionState state = StepProvisionState.NotStarted;
        private static string status = string.Empty;

        // Blocks a native host call until the payload is usable. Called
        // on every STEP import: Ready is a single field read.
        public static void EnsureReady()
        {
            if (state == StepProvisionState.NotStarted)
            {
                // InitializeOnLoad runs before the first import of a
                // domain; this is only a safety net for an import that
                // somehow precedes it.
                Kickoff();
            }
            switch (state)
            {
                case StepProvisionState.Ready:
                    return;
                case StepProvisionState.Running:
                    throw new InvalidOperationException(
                        "the OCCT payload is still provisioning "
                        + $"({status}); wait for it to finish and reimport");
                case StepProvisionState.Failed:
                    throw new InvalidOperationException(
                        "the OCCT payload failed to provision: "
                        + $"{status} — fix the cause (Unity reloads the "
                        + "package on script changes) and reimport");
                default:
                    throw new InvalidOperationException(
                        "the OCCT payload has not been provisioned yet");
            }
        }

        [InitializeOnLoadMethod]
        private static void KickoffOnLoad()
        {
            Kickoff();
        }

        private static void Kickoff()
        {
            if (state != StepProvisionState.NotStarted)
            {
                return;
            }
            try
            {
                string packageRoot = PackageRoot();
                string payloadDir = Path.Combine(
                    packageRoot, "Runtime", "Plugins", "occt", NativeMethods.PluginArch(), "lib");
                string markerPath = payloadDir + ".provenance.toml";
                StepFetchConfig config = StepFetchConfig.Load(packageRoot);

                StepProvenance marker = StepProvenance.Read(markerPath);
                if (marker != null && marker.Matches(config))
                {
                    // Cheap domain-reload check: nothing downloads when
                    // the payload already certifies the pinned archive.
                    state = StepProvisionState.Ready;
                    return;
                }

                // A missing marker or a mismatch (another release, a
                // half-finished run, the pre-provisioning staged
                // payload) means this directory does not certify the
                // package's pin: re-extract from the verified archive.
                // The deletion only happens after the new zip has passed
                // its hash check, so a failed run cannot leave the
                // payload destroyed without a path to recovery.
                BeginDownload(config, payloadDir, markerPath);
            }
            catch (Exception exception)
            {
                state = StepProvisionState.Failed;
                status = exception.Message;
                Debug.LogError(
                    "[com.greyhound.step] OCCT payload provisioning could not start: "
                    + exception.Message);
            }
        }

        // The staged zip's name inside download_location (file mode
        // only; memory mode never names a file).
        private const string ZipFileName = "occt-payload.zip";

        private static void BeginDownload(
            StepFetchConfig config, string payloadDir, string markerPath)
        {
            if (config.DownloadLocation.Length == 0)
            {
                // Memory mode (the tracked default): the whole archive
                // lands in a managed byte array (DownloadHandlerBuffer),
                // never on disk — the behavior for machines whose IT
                // policy locks down temp directories, and there is no
                // cleanup step. Cost is a transient managed allocation
                // of the archive size — tens of MB, once per release.
                var request = new UnityWebRequest(config.Url, UnityWebRequest.kHttpVerbGET)
                {
                    downloadHandler = new DownloadHandlerBuffer(),
                };
                UnityWebRequestAsyncOperation operation = request.SendWebRequest();
                state = StepProvisionState.Running;
                status = $"downloading {config.Url}";
                _ = FinishAsync(request, operation, config, payloadDir, markerPath);
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
            string zipPath = Path.Combine(config.DownloadLocation, ZipFileName);
            var fileRequest = new UnityWebRequest(config.Url, UnityWebRequest.kHttpVerbGET)
            {
                downloadHandler = new DownloadHandlerFile(zipPath) { removeFileOnAbort = true },
            };
            UnityWebRequestAsyncOperation fileOperation = fileRequest.SendWebRequest();
            state = StepProvisionState.Running;
            status = $"downloading {config.Url}";
            _ = FinishFileAsync(fileRequest, fileOperation, config, payloadDir, markerPath, zipPath);
        }

        private static async Task FinishAsync(
            UnityWebRequest request,
            UnityWebRequestAsyncOperation operation,
            StepFetchConfig config,
            string payloadDir,
            string markerPath)
        {
            try
            {
                await AwaitDownload(operation);
                if (request.result != UnityWebRequest.Result.Success)
                {
                    throw new InvalidOperationException(
                        $"download of {config.Url} failed "
                        + $"({request.responseCode}): {request.error}");
                }
                byte[] zipBytes = request.downloadHandler.data;
                VerifyHash(config, Sha256Hex(zipBytes));
                int files = Extract(zipBytes, StagingDir(payloadDir));
                Install(config, payloadDir, markerPath, files);
            }
            catch (Exception exception)
            {
                Fail(exception);
            }
        }

        private static async Task FinishFileAsync(
            UnityWebRequest request,
            UnityWebRequestAsyncOperation operation,
            StepFetchConfig config,
            string payloadDir,
            string markerPath,
            string zipPath)
        {
            try
            {
                await AwaitDownload(operation);
                if (request.result != UnityWebRequest.Result.Success)
                {
                    throw new InvalidOperationException(
                        $"download of {config.Url} failed "
                        + $"({request.responseCode}): {request.error}");
                }
                VerifyHash(config, Sha256Hex(zipPath));
                int files = Extract(zipPath, StagingDir(payloadDir));
                Install(config, payloadDir, markerPath, files);
            }
            catch (Exception exception)
            {
                Fail(exception);
            }
            finally
            {
                // The staged zip is disposable; failing to delete it
                // must not fail the provisioning.
                TryDelete(zipPath);
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

        private static void VerifyHash(StepFetchConfig config, string actual)
        {
            if (!string.Equals(actual, config.Sha256, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException(
                    "the downloaded OCCT payload hash " + actual
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

        private static void Install(
            StepFetchConfig config, string payloadDir, string markerPath, int files)
        {
            if (Directory.Exists(payloadDir))
            {
                Directory.Delete(payloadDir, recursive: true);
            }
            Directory.Move(StagingDirName(payloadDir), payloadDir);

            StepProvenance.ForConfig(config).Write(markerPath);
            AssetDatabase.Refresh();
            state = StepProvisionState.Ready;
            Debug.Log(
                "[com.greyhound.step] OCCT payload provisioned: "
                + $"{files} files into {payloadDir}");
        }

        private static string StagingDirName(string payloadDir)
        {
            return payloadDir + ".staging";
        }

        private static void Fail(Exception exception)
        {
            state = StepProvisionState.Failed;
            status = exception.Message;
            Debug.LogError(
                "[com.greyhound.step] OCCT payload provisioning failed: "
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
