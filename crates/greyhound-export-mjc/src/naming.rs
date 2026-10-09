use std::collections::HashSet;

/// Maps a raw STEP name onto an MJCF-safe identifier: every character
/// outside `[A-Za-z0-9_.-]` becomes `_`. MJCF names are free-form strings,
/// so this is exporter policy for predictable, comment-safe names, not a
/// MuJoCo requirement.
pub fn sanitize(raw: &str) -> String {
    raw.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// Makes a raw name safe inside an XML comment: line breaks flatten to
/// spaces and the `--` sequence (illegal inside a comment) collapses to `-`.
pub fn comment_text(raw: &str) -> String {
    let flattened = raw.replace(['\r', '\n'], " ");
    let mut text = String::with_capacity(flattened.len());
    let mut previous_dash = false;
    for character in flattened.chars() {
        if character == '-' {
            if !previous_dash {
                text.push('-');
            }
            previous_dash = true;
        } else {
            text.push(character);
            previous_dash = false;
        }
    }
    text
}

/// Allocates unique MJCF body names: names must be unique among elements of
/// the same type and are case-sensitive (modeling guide, Naming elements).
pub struct NameAllocator {
    used: HashSet<String>,
}

impl NameAllocator {
    pub fn new() -> Self {
        Self {
            used: HashSet::new(),
        }
    }

    /// Sanitizes `raw` (falling back to `fallback` when the result is empty)
    /// and dedupes collisions with `_2`, `_3`, ... suffixes. STEP files
    /// legitimately reuse names across instances, so deduping is mandatory.
    pub fn allocate(&mut self, raw: &str, fallback: &str) -> String {
        let mut candidate = sanitize(raw);
        if candidate.is_empty() {
            candidate = sanitize(fallback);
        }
        let mut attempt = candidate.clone();
        let mut suffix = 2u32;
        while self.used.contains(&attempt) {
            attempt = format!("{candidate}_{suffix}");
            suffix += 1;
        }
        self.used.insert(attempt.clone());
        attempt
    }
}

impl Default for NameAllocator {
    fn default() -> Self {
        Self::new()
    }
}
