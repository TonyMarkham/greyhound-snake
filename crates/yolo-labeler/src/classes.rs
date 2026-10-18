use std::path::Path;

use crate::image_set::ImageSet;

use gpui::{Rgba, SharedString, rgb};

/// Class list file, one class name per line, living beside the images.
pub(crate) const CLASSES_FILE: &str = "classes.txt";
/// Seeded class when the folder has no `classes.txt` yet.
pub(crate) const DEFAULT_CLASS: &str = "object";

/// Fixed review palette; class ids cycle through it by modulo so even
/// high ids stay visible.
const PALETTE: [u32; 10] = [
    0xe6194b, 0x3cb44b, 0xffe119, 0x4363d8, 0xf58231, 0x911eb4, 0x42d4f4, 0xf032e6, 0x9a6324,
    0x808000,
];

pub(crate) fn class_color(class: u32) -> Rgba {
    let index = usize::try_from(class).unwrap_or(0);
    rgb(PALETTE[index % PALETTE.len()])
}

/// Loads `classes.txt` from the folder, or seeds it with
/// [`DEFAULT_CLASS`] when missing or empty so the folder is immediately
/// usable. Hand-edit the file (one name per line) to extend the list.
pub(crate) fn load_or_seed(dir: &Path) -> std::io::Result<Vec<SharedString>> {
    let path = dir.join(CLASSES_FILE);
    if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        let names: Vec<SharedString> = text
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| SharedString::from(name.to_string()))
            .collect();
        if !names.is_empty() {
            return Ok(names);
        }
    }
    std::fs::write(&path, format!("{DEFAULT_CLASS}\n"))?;
    Ok(vec![SharedString::from(DEFAULT_CLASS)])
}

/// Display names ordered by class id slot, one slot per id from 0 to
/// the maximum; ids missing from the source keep `class N` placeholders
/// so gap ids cannot silently display the wrong name.
pub(crate) fn names_by_slot(slots: Vec<(u32, String)>) -> Vec<SharedString> {
    let max_slot = slots.iter().map(|(slot, _)| *slot).max().unwrap_or(0);
    let len = usize::try_from(max_slot).map(|slot| slot + 1).unwrap_or(0);
    let mut names: Vec<SharedString> = (0..len as u64)
        .map(|slot| SharedString::from(format!("class {slot}")))
        .collect();
    for (slot, name) in slots {
        if let Some(index) = usize::try_from(slot).ok().filter(|index| *index < len) {
            names[index] = SharedString::from(name);
        }
    }
    names
}

/// Rewrites `classes.txt` from the in-memory list, one name per line.
pub(crate) fn save(dir: &Path, names: &[SharedString]) -> std::io::Result<()> {
    let mut text = String::new();
    for name in names {
        text.push_str(name);
        text.push('\n');
    }
    std::fs::write(dir.join(CLASSES_FILE), text)
}

/// Extends `names` in place to cover every class id present in the
/// set's label files (resolved through the set's own layout), appending
/// `class N` placeholders for the unknown ones. Returns whether the
/// list grew; persisting that is the caller's decision — the caller
/// knows whether a tree `data.yaml` forbids writing `classes.txt`.
pub(crate) fn extend_to_cover(names: &mut Vec<SharedString>, set: &ImageSet) -> bool {
    let mut max_id: Option<u32> = None;
    for image in set.images() {
        // Labels resolved through the set, never the image files
        // themselves: reading those as text fails to decode and would
        // silently find no classes.
        let Ok(text) = std::fs::read_to_string(set.label_path_for(image)) else {
            continue;
        };
        for box_ in crate::yolo::parse_labels(&text) {
            max_id = Some(match max_id {
                Some(current) => current.max(box_.class),
                None => box_.class,
            });
        }
    }
    let Some(max_id) = max_id else {
        return false;
    };
    let needed = usize::try_from(max_id)
        .map(|id| id + 1)
        .unwrap_or(names.len());
    let changed = names.len() < needed;
    while names.len() < needed {
        names.push(SharedString::from(format!("class {}", names.len())));
    }
    changed
}
