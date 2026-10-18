use std::path::{Path, PathBuf};

use gpui::SharedString;
use serde::Deserialize;

use crate::classes;

/// The dataset tree's `data.yaml` carries the default Ultralytics
/// keys: `path:`, `train:`, `val:`, `names:` (rig slot order), plus
/// `channels:`. It lives one directory above the target folder — the
/// target itself holds only `images/` and `labels/`. While the yaml
/// exists there, `classes.txt` is never written.
pub(crate) const DATA_YAML_FILE: &str = "data.yaml";

#[derive(Deserialize)]
struct DataYaml {
    #[serde(default)]
    names: Option<YamlNames>,
}

/// Ultralytics accepts `names:` both as a list (slot = index) and as a
/// mapping (`0: Left`).
#[derive(Deserialize)]
#[serde(untagged)]
enum YamlNames {
    List(Vec<String>),
    Map(std::collections::BTreeMap<u32, String>),
}

fn yaml_err(err: serde_yaml::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err)
}

/// Locates the tree's `data.yaml`: one directory above the target
/// folder. The target itself holds only `images/` and `labels/`.
pub(crate) fn find_yaml(dir: &Path) -> Option<PathBuf> {
    let above = dir.parent()?.join(DATA_YAML_FILE);
    above.is_file().then_some(above)
}

/// Reads `names:` from the given `data.yaml`. `None` when unparsable
/// or the key is absent — the caller falls through to the next source.
pub(crate) fn load_names(yaml_path: &Path) -> Option<Vec<SharedString>> {
    let text = std::fs::read_to_string(yaml_path).ok()?;
    let yaml: DataYaml = serde_yaml::from_str(&text).ok()?;
    match yaml.names? {
        YamlNames::List(list) => Some(list.into_iter().map(SharedString::from).collect()),
        YamlNames::Map(map) => Some(classes::names_by_slot(map.into_iter().collect())),
    }
}

/// Rewrites `names:` (as a list, slot = index) in the given
/// `data.yaml`, leaving every other key as parsed. Comment formatting
/// is not preserved — the yaml is machine-written by the harness.
pub(crate) fn update_names(yaml_path: &Path, names: &[SharedString]) -> std::io::Result<()> {
    let text = std::fs::read_to_string(yaml_path)?;
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&text).map_err(yaml_err)?;
    let Some(mapping) = doc.as_mapping_mut() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "data.yaml is not a mapping",
        ));
    };
    mapping.insert(
        serde_yaml::Value::String("names".to_string()),
        serde_yaml::Value::Sequence(
            names
                .iter()
                .map(|name| serde_yaml::Value::String(name.to_string()))
                .collect(),
        ),
    );
    std::fs::write(yaml_path, serde_yaml::to_string(&doc).map_err(yaml_err)?)
}
