using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Greyhound.Step
{
    // The sibling `<dir>.provenance.toml` marker: the provisioner's
    // ownership claim on the payload directory — the same contract
    // crates/extern-fetch writes (provenance.rs), read back by both
    // sides. A directory is trusted (and left untouched) only when the
    // marker exists and matches the fetch config entry; ownership is the
    // coordinates plus hash, not the tool that wrote it, so `just
    // fetch-occt` and this provisioner honor each other's markers on the
    // shared destination.
    internal sealed class StepProvenance
    {
        // The tool name this provisioner records — distinct from
        // extern-fetch's own markers, and deliberately not part of
        // Matches.
        public const string ToolName = "com.greyhound.step";

        public string Tool { get; }
        public string Group { get; }
        public string Role { get; }
        public string Repo { get; }
        public string Tag { get; }
        public string Url { get; }
        public string Sha256 { get; }

        private StepProvenance(
            string tool, string group, string role,
            string repo, string tag, string url, string sha256)
        {
            Tool = tool;
            Group = group;
            Role = role;
            Repo = repo;
            Tag = tag;
            Url = url;
            Sha256 = sha256;
        }

        public static StepProvenance ForConfig(StepFetchConfig config)
        {
            return new StepProvenance(
                ToolName, StepFetchConfig.Group, StepFetchConfig.Role,
                config.Repo, config.Tag, config.Url, config.Sha256);
        }

        // Null when no marker exists (nothing provisioned here yet) —
        // the caller decides what an unmarked or mismatched directory
        // means; a corrupt marker throws (same as extern-fetch).
        public static StepProvenance Read(string path)
        {
            if (!File.Exists(path))
            {
                return null;
            }
            Dictionary<string, Dictionary<string, string>> tables =
                StepToml.Parse(File.ReadAllText(path));
            if (!tables.TryGetValue("", out Dictionary<string, string> root))
            {
                throw new InvalidOperationException(
                    $"{path} is not a valid provenance marker: it has no fields");
            }
            return new StepProvenance(
                Require(root, "tool", path),
                Require(root, "group", path),
                Require(root, "role", path),
                Require(root, "repo", path),
                Require(root, "tag", path),
                Require(root, "url", path),
                Require(root, "sha256", path));
        }

        public bool Matches(StepFetchConfig config)
        {
            return Group == StepFetchConfig.Group
                && Role == StepFetchConfig.Role
                && Repo == config.Repo
                && Tag == config.Tag
                && Url == config.Url
                // Hash comparison is case-insensitive: release pages
                // print lowercase, but a hand-pinned uppercase hash must
                // still match.
                && string.Equals(Sha256, config.Sha256, StringComparison.OrdinalIgnoreCase);
        }

        public void Write(string path)
        {
            var text = new StringBuilder();
            AppendField(text, "tool", Tool);
            AppendField(text, "group", Group);
            AppendField(text, "role", Role);
            AppendField(text, "repo", Repo);
            AppendField(text, "tag", Tag);
            AppendField(text, "url", Url);
            AppendField(text, "sha256", Sha256);
            File.WriteAllText(path, text.ToString());
        }

        // The marker's fields are plain names, urls, and hex hashes: no
        // characters ever need escaping, so the writer emits the value
        // verbatim and the reader's no-escapes rule stays honest.
        private static void AppendField(StringBuilder text, string key, string value)
        {
            text.Append(key).Append(" = \"").Append(value).Append("\"\n");
        }

        private static string Require(
            Dictionary<string, string> table, string key, string path)
        {
            if (!table.TryGetValue(key, out string value) || value.Length == 0)
            {
                throw new InvalidOperationException(
                    $"{path} is not a valid provenance marker: {key} is missing or empty");
            }
            return value;
        }
    }
}
