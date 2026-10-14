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
    // Runtime/Plugins/occt/x86_64/lib at editor load (refactor-repo.md
    // D5/D6): the UPM package ships light and downloads the pinned
    // release archive instead. UPM has no install-time hook, so the
    // handshake point is editor assembly load; a STEP import that
    // arrives before provisioning is done fails with a clear message
    // (StepImporter calls EnsureReady), never with a cryptic loader
    // error.
    //
    // The archive, its hash, and the destination contract come from the
    // staged copy of crates/extern-fetch/config.toml (refactor-repo.md
    // D8); `just fetch-occt` provisions the same destination from the
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
        private const string PayloadRelPath = "Runtime/Plugins/occt/x86_64/lib";

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
                    packageRoot, "Runtime", "Plugins", "occt", "x86_64", "lib");
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

        private static void BeginDownload(
            StepFetchConfig config, string payloadDir, string markerPath)
        {
            // The zip is staged outside the package: a partially
            // downloaded zip inside the package would be imported as
            // assets on the next focus switch.
            string tempRoot = Path.Combine(Path.GetTempPath(), "com.greyhound.step");
            Directory.CreateDirectory(tempRoot);
            string zipPath = Path.Combine(tempRoot, "occt-payload.zip");
            var request = new UnityWebRequest(config.Url, UnityWebRequest.kHttpVerbGET)
            {
                downloadHandler = new DownloadHandlerFile(zipPath) { removeFileOnAbort = true },
            };
            UnityWebRequestAsyncOperation operation = request.SendWebRequest();
            state = StepProvisionState.Running;
            status = $"downloading {config.Url}";
            _ = FinishAsync(request, operation, config, payloadDir, markerPath, zipPath);
        }

        private static async Task FinishAsync(
            UnityWebRequest request,
            UnityWebRequestAsyncOperation operation,
            StepFetchConfig config,
            string payloadDir,
            string markerPath,
            string zipPath)
        {
            try
            {
                // Task.Yield resumes on Unity's synchronization context,
                // so every step below runs on the main thread.
                while (!operation.isDone)
                {
                    await Task.Yield();
                }
                if (request.result != UnityWebRequest.Result.Success)
                {
                    throw new InvalidOperationException(
                        $"download of {config.Url} failed "
                        + $"({request.responseCode}): {request.error}");
                }

                string actual = Sha256Hex(zipPath);
                if (!string.Equals(actual, config.Sha256, StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidOperationException(
                        "the downloaded OCCT payload hash " + actual
                        + " does not match the pinned sha256 " + config.Sha256
                        + "; nothing was extracted");
                }

                // Extraction targets a staging directory beside the
                // payload and the result is moved into place (same
                // volume), so a reader never sees a half-populated lib/
                // and the move never crosses a filesystem boundary.
                string stagingDir = payloadDir + ".staging";
                if (Directory.Exists(stagingDir))
                {
                    Directory.Delete(stagingDir, recursive: true);
                }
                int files = Extract(zipPath, stagingDir);
                if (Directory.Exists(payloadDir))
                {
                    Directory.Delete(payloadDir, recursive: true);
                }
                Directory.Move(stagingDir, payloadDir);

                StepProvenance.ForConfig(config).Write(markerPath);
                TryDelete(zipPath);
                AssetDatabase.Refresh();
                state = StepProvisionState.Ready;
                Debug.Log(
                    "[com.greyhound.step] OCCT payload provisioned: "
                    + $"{files} files into {payloadDir}");
            }
            catch (Exception exception)
            {
                state = StepProvisionState.Failed;
                status = exception.Message;
                Debug.LogError(
                    "[com.greyhound.step] OCCT payload provisioning failed: "
                    + exception.Message);
            }
            finally
            {
                // The downloaded zip is disposable; failing to delete it
                // must not fail the provisioning.
                TryDelete(zipPath);
            }
        }

        private static string Sha256Hex(string path)
        {
            // Streaming: the archive is tens to hundreds of MB; never
            // buffer it whole in memory just to hash it.
            using (var sha = SHA256.Create())
            using (FileStream stream = File.OpenRead(path))
            {
                var buffer = new byte[1024 * 1024];
                int read;
                while ((read = stream.Read(buffer, 0, buffer.Length)) > 0)
                {
                    sha.TransformBlock(buffer, 0, read, null, 0);
                }
                sha.TransformFinalBlock(Array.Empty<byte>(), 0, 0);
                var hex = new StringBuilder(64);
                foreach (byte b in sha.Hash)
                {
                    hex.Append(b.ToString("x2"));
                }
                return hex.ToString();
            }
        }

        // Extracts the verified archive into dest, mirroring
        // crates/extern-fetch's archive.rs rules: refuse any entry that
        // escapes the extraction root (zip-slip), return the count of
        // files written.
        private static int Extract(string zipPath, string dest)
        {
            Directory.CreateDirectory(dest);
            using (var archive = new ZipArchive(
                File.OpenRead(zipPath), ZipArchiveMode.Read, leaveOpen: false))
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
