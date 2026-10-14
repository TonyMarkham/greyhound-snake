using System;
using System.Collections.Generic;
using System.IO;

namespace Greyhound.Step
{
    // Strict subset of TOML, sufficient for the two files the C#
    // provisioner parses (the staged fetch config and the provenance
    // markers): whole-line comments (#), [section.headers], root-level
    // keys (markers have no section header), and bare keys with quoted
    // basic-string values. Anything outside the subset — unquoted
    // values, inline comments after a value, escaped characters, dotted
    // keys — fails loudly with the line number instead of being
    // silently skipped: a config the parser cannot fully understand
    // must never look like a missing entry.
    internal sealed class StepToml
    {
        // Section path (e.g. "occt.windows.payload") to key to value.
        // Root-level keys (before any header, as in a provenance marker)
        // land in the "" table.
        private readonly Dictionary<string, Dictionary<string, string>> tables =
            new Dictionary<string, Dictionary<string, string>>(StringComparer.Ordinal);

        private string currentSection;

        public static Dictionary<string, Dictionary<string, string>> Parse(string text)
        {
            var parser = new StepToml();
            using (var reader = new StringReader(text))
            {
                int lineNumber = 0;
                while (reader.ReadLine() is string line)
                {
                    lineNumber++;
                    parser.ParseLine(line, lineNumber);
                }
            }
            return parser.tables;
        }

        private StepToml() { }

        private void ParseLine(string line, int lineNumber)
        {
            string trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed.StartsWith("#", StringComparison.Ordinal))
            {
                return;
            }
            if (trimmed.StartsWith("[", StringComparison.Ordinal))
            {
                ParseHeader(trimmed, lineNumber);
                return;
            }
            ParseKeyValue(trimmed, lineNumber);
        }

        private void ParseHeader(string line, int lineNumber)
        {
            if (!line.EndsWith("]", StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: unterminated section header {line}");
            }
            string name = line.Substring(1, line.Length - 2).Trim();
            if (name.Length == 0 || name.Contains("[") || name.Contains("]")
                || name.Contains("\"") || name.Contains(" "))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: unsupported section header {line}");
            }
            if (tables.ContainsKey(name))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: duplicate section [{name}]");
            }
            tables.Add(name, new Dictionary<string, string>(StringComparer.Ordinal));
            currentSection = name;
        }

        private void ParseKeyValue(string line, int lineNumber)
        {
            // A key/value before any section header belongs to the root
            // table — the provenance marker's shape.
            Dictionary<string, string> table = currentSection == null
                ? RootTable()
                : tables[currentSection];
            int equals = line.IndexOf('=');
            if (equals <= 0)
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: not a key = \"value\" line: {line}");
            }
            string key = line.Substring(0, equals).Trim();
            if (key.Length == 0 || key.Contains("\"") || key.Contains(" "))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: unsupported key {key}");
            }
            string value = ParseString(line.Substring(equals + 1).Trim(), lineNumber);
            if (table.ContainsKey(key))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: duplicate key {key}");
            }
            table.Add(key, value);
        }

        private Dictionary<string, string> RootTable()
        {
            if (!tables.TryGetValue("", out Dictionary<string, string> root))
            {
                root = new Dictionary<string, string>(StringComparer.Ordinal);
                tables.Add("", root);
            }
            return root;
        }

        private static string ParseString(string value, int lineNumber)
        {
            if (value.Length < 2 || !value.StartsWith("\"", StringComparison.Ordinal)
                || !value.EndsWith("\"", StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"line {lineNumber}: value must be a double-quoted string, got {value}");
            }
            string inner = value.Substring(1, value.Length - 2);
            // No escape sequences: the parser covers exactly the values
            // this repo writes (urls, hashes, names, paths); a backslash
            // or an inner quote means the file is not the shape we
            // expect. The backslash hint names the common case: a
            // Windows path typed with `\`, which TOML reads as an
            // escape character.
            if (inner.Contains("\"") || inner.Contains("\\"))
            {
                throw new InvalidOperationException(
                    inner.Contains("\\")
                        ? $"line {lineNumber}: unsupported characters in {value} "
                          + "(a backslash is a TOML escape character; write paths "
                          + "with forward slashes)"
                        : $"line {lineNumber}: unsupported characters in {value}");
            }
            return inner;
        }
    }
}
