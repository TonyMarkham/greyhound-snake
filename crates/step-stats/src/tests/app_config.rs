use crate::{app_config::AppConfig, config_error::ConfigError};
use std::fs;

#[test]
fn library_path_is_relative_to_config() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let shim = root.path().join("shim");
    fs::create_dir(&shim)?;
    // Every per-OS section is written; the build target's is the one
    // that resolves — the config is host-authored, the selection is not.
    let native_dirs = [
        ("linux", "native/linux"),
        ("windows", "native/windows"),
        ("macos", "native/macos"),
    ];
    let mut config_text = String::new();
    for (os, dir) in native_dirs {
        config_text.push_str(&format!(
            "[occt.{os}]\nshim_dir = 'shim'\nlibrary_dir = '{dir}'\n\n"
        ));
        fs::create_dir_all(root.path().join(dir))?;
    }
    let config = root.path().join("config.toml");
    fs::write(&config, config_text)?;
    let loaded = AppConfig::load(&config)?;
    let expected = root.path().join(match os_name() {
        "windows" => "native/windows",
        "macos" => "native/macos",
        _ => "native/linux",
    });
    assert_eq!(loaded.occt.library_dir, expected.canonicalize()?);
    assert_eq!(loaded.occt.shim_dir, shim.canonicalize()?);
    Ok(())
}

#[test]
fn missing_os_section_is_the_platform_gate() -> Result<(), Box<dyn std::error::Error>> {
    // The section for THIS build target does not exist, so the load
    // fails naming the section instead of resolving paths.
    let root = tempfile::tempdir()?;
    let config = root.path().join("config.toml");
    fs::write(&config, "[occt]\n")?;
    let error = match AppConfig::load(&config) {
        Err(error) => error,
        Ok(_) => panic!("config without a section for this OS must not load"),
    };
    assert!(
        error.to_string().contains(&format!("[occt.{}]", os_name())),
        "{error}"
    );
    Ok(())
}

fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[test]
fn unknown_key_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let config = root.path().join("config.toml");
    fs::write(
        &config,
        "[occt]\nshim_dir = '.'\nshim_path = 'unused.so'\nlibary_dir = '.'\n",
    )?;
    assert!(matches!(
        AppConfig::load(&config),
        Err(ConfigError::Parse { .. })
    ));
    Ok(())
}
