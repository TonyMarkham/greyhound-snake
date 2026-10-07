use crate::{app_config::AppConfig, config_error::ConfigError};
use std::fs;

#[test]
fn library_path_is_relative_to_config() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let library = root.path().join("native");
    fs::create_dir(&library)?;
    let shim = library.join("libgreyhound_occt.so");
    fs::write(&shim, b"")?;
    let config = root.path().join("config.toml");
    fs::write(
        &config,
        "[occt]\nlibrary_dir = 'native'\nshim_path = 'native/libgreyhound_occt.so'\n",
    )?;
    let loaded = AppConfig::load(&config)?;
    assert_eq!(loaded.occt.library_dir, library.canonicalize()?);
    assert_eq!(loaded.occt.shim_path, shim.canonicalize()?);
    Ok(())
}

#[test]
fn unknown_key_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let config = root.path().join("config.toml");
    fs::write(
        &config,
        "[occt]\nlibrary_dir = '.'\nshim_path = 'unused.so'\nlibary_dir = '.'\n",
    )?;
    assert!(matches!(
        AppConfig::load(&config),
        Err(ConfigError::Parse { .. })
    ));
    Ok(())
}
