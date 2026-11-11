using System;
using System.Collections.Generic;
using System.IO;

using UnityEngine;

namespace Greyhound.Step
{
    // The pins the C# provisioner fetches with: the tracked copy of
    // crates/extern-fetch/config.toml at Editor/occt-fetch.toml. One
    // source of pins — `just sync-fetch-config` re-copies it after every
    // pin change; until it runs, the package keeps the old pins. The
    // provisioner reads one <group>.<platform>.payload entry per folder it
    // owns (occt, greyhound_host, greyhound_shim); the dev entries (shim
    // build inputs) are meaningless in Unity and are ignored.
    internal sealed class StepFetchConfig
    {
        public const string Role = "payload";
        public const string StagedFileName = "occt-fetch.toml";

        // The config group this entry was loaded from ("occt",
        // "greyhound_host", "greyhound_shim"): the staged config holds
        // every group, and the provisioner loads one entry per payload
        // directory it owns.
        public string Group { get; }
        public string Repo { get; }
        public string Tag { get; }
        public string Url { get; }
        public string Sha256 { get; }

        // Empty: the download is held in memory only (also the right
        // behavior for machines whose IT policy locks down temp
        // directories). Set: an absolute path to an existing writable
        // directory the download stages its zip in before extraction —
        // a per-machine override of the tracked-default-empty key.
        public string DownloadLocation { get; }

        private StepFetchConfig(string group, string repo, string tag, string url, string sha256,string downloadLocation)
        {
            Group = group;
            Repo = repo;
            Tag = tag;
            Url = url;
            Sha256 = sha256;
            DownloadLocation = downloadLocation;
        }

        // Loads one group's entry from the staged config for the platform
        // the editor runs on. The platform key is the same
        // "windows"/"linux"/"macos" the extern-fetch config sections use;
        // the entry for the current platform selects the archive.
        //
        // required = true (OCCT): a missing group or platform section
        // throws — the importer cannot run without it. required = false
        // (our own host/shim DLLs): a missing group or platform section
        // returns null, meaning "no release for this platform; use what
        // a local cargo build put in the folder".
        public static StepFetchConfig Load(string packageRoot, string groupName, bool required)
        {
            string path = Path.Combine(packageRoot, "Editor", StagedFileName);
            if (!File.Exists(path))
            {
                throw new InvalidOperationException(
                    $"the package's fetch config is missing: {path}; "
                    + "restore it with 'just sync-fetch-config'");
            }
            Dictionary<string, Dictionary<string, string>> tables =
                StepToml.Parse(File.ReadAllText(path));

            // The root-level download_location (before any section
            // header): empty = in-memory download; set = absolute path
            // to an existing writable directory. Relative paths are
            // rejected here — there is no honest root to resolve them
            // against, and the key's meaning is machine-local.
            string downloadLocation = tables.TryGetValue("", out Dictionary<string, string> root)
                && root.TryGetValue("download_location", out string location)
                ? location
                : "";
            if (downloadLocation.Length > 0 && !Path.IsPathRooted(downloadLocation))
            {
                throw new InvalidOperationException(
                    $"{path}: download_location must be an absolute path with forward "
                    + $"slashes (got {downloadLocation}); empty string = in-memory download");
            }

            if (!tables.TryGetValue(groupName, out Dictionary<string, string> group))
            {
                if (!required)
                {
                    return null;
                }
                throw new InvalidOperationException($"[{groupName}] is missing from {path}");
            }
            string repo = Require(group, "repo", path);
            string tag = Require(group, "tag", path);

            string section = $"{groupName}.{PlatformKey()}.{Role}";
            if (!tables.TryGetValue(section, out Dictionary<string, string> entry))
            {
                if (!required)
                {
                    return null;
                }
                throw new InvalidOperationException(
                    $"[{section}] is missing from {path}; "
                    + "no payload is configured for this platform");
            }
            string url = Require(entry, "url", path);
            // An empty pin is the unpinned state (the extern-fetch
            // convention): it fails here with the pinning instructions,
            // not with a generic missing-key message.
            string sha256 = entry.TryGetValue("sha256", out string pinned) ? pinned : "";

            // Same-tag invariant, mirrored from the
            // extern-fetch validator: the url must name this group's own
            // repo and tag.
            string prefix = $"https://github.com/{repo}/releases/download/{tag}/";
            if (!url.StartsWith(prefix, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"{section}: url must start with {prefix} (same-tag invariant)");
            }
            if (!IsSha256Hex(sha256))
            {
                throw new InvalidOperationException(
                    $"{section}: sha256 is not pinned in {path}; pin the archive's "
                    + "64-hex-digit sha256 in crates/extern-fetch/config.toml and "
                    + "run 'just sync-fetch-config'");
            }
            return new StepFetchConfig(groupName, repo, tag, url, sha256, downloadLocation);
        }

        // The editor platform as the extern-fetch config's OS section key.
        // Player platforms never provision (the payload ships with a
        // built player); anything unrecognized fails loudly.
        public static string PlatformKey()
        {
            switch (Application.platform)
            {
                case RuntimePlatform.WindowsEditor:
                    return "windows";
                case RuntimePlatform.LinuxEditor:
                    return "linux";
                case RuntimePlatform.OSXEditor:
                    return "macos";
                default:
                    throw new InvalidOperationException(
                        $"no OCCT payload provisioning for platform {Application.platform}");
            }
        }

        private static string Require(
            Dictionary<string, string> table, string key, string path)
        {
            if (!table.TryGetValue(key, out string value) || value.Length == 0)
            {
                throw new InvalidOperationException($"{key} is missing or empty in {path}");
            }
            return value;
        }

        private static bool IsSha256Hex(string text)
        {
            if (text.Length != 64)
            {
                return false;
            }
            foreach (char character in text)
            {
                if (!Uri.IsHexDigit(character))
                {
                    return false;
                }
            }
            return true;
        }
    }
}
