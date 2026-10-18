use std::path::{Path, PathBuf};

use crate::yolo;

/// Extensions accepted by the folder scan, compared lowercased. The set
/// mirrors the decoders enabled for the `image` dependency.
const IMAGE_EXTENSIONS: &[&str] = &["bmp", "gif", "jpg", "jpeg", "png", "tif", "tiff", "webp"];

/// The images of one folder plus the current position in it, and the
/// rule for finding each image's label file.
///
/// Two layouts are recognized:
/// - **Ultralytics dataset run** — the folder has an `images/` child:
///   frames are scanned there and labels resolve to the sibling
///   `labels/` folder, paired by stem (Ultralytics' own path
///   substitution, the dataset format's pairing mechanism).
/// - **Flat folder** — images sit beside their `<image>.txt` labels.
///
/// Scanning is case-insensitive on the extension and sorted by path,
/// so navigation order is stable across runs.
pub(crate) struct ImageSet {
    images: Vec<PathBuf>,
    labels_dir: Option<PathBuf>,
    index: usize,
}

impl ImageSet {
    pub(crate) fn scan(dir: &Path) -> std::io::Result<Self> {
        let images_dir = dir.join("images");
        let (image_root, labels_dir) = if images_dir.is_dir() {
            (images_dir, Some(dir.join("labels")))
        } else {
            (dir.to_path_buf(), None)
        };
        let mut images = Vec::new();
        for entry in std::fs::read_dir(image_root)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let is_image = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                });
            if is_image {
                images.push(path);
            }
        }
        images.sort();
        Ok(Self {
            images,
            labels_dir,
            index: 0,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.images.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    pub(crate) fn position(&self) -> usize {
        self.index
    }

    pub(crate) fn current(&self) -> Option<&Path> {
        self.images.get(self.index).map(PathBuf::as_path)
    }

    pub(crate) fn images(&self) -> &[PathBuf] {
        &self.images
    }

    /// The label file for an image under this set's layout.
    pub(crate) fn label_path_for(&self, image: &Path) -> PathBuf {
        match &self.labels_dir {
            Some(labels_dir) => match image.file_stem().and_then(|stem| stem.to_str()) {
                Some(stem) => labels_dir.join(format!("{stem}.txt")),
                None => yolo::label_path_for(image),
            },
            None => yolo::label_path_for(image),
        }
    }

    /// Cycles: past the last image wraps to the first.
    pub(crate) fn next(&mut self) {
        if !self.is_empty() {
            self.index = (self.index + 1) % self.images.len();
        }
    }

    /// Cycles: before the first image wraps to the last.
    pub(crate) fn prev(&mut self) {
        if !self.is_empty() {
            self.index = if self.index == 0 {
                self.images.len() - 1
            } else {
                self.index - 1
            };
        }
    }
}
