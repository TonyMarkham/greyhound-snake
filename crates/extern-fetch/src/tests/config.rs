use crate::{config::Config, host_os::HostOs};

use std::{fs, path::Path};
use tempfile::TempDir;

fn config_text(sha256: &str, dest: &str) -> String {
    format!(
        "[occt]\nrepo = \"example/occt-dev\"\ntag = \"v9\"\n\n[occt.windows.dev]\nurl = \"https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip\"\nsha256 = \"{sha256}\"\ndest = \"{dest}\"\n"
    )
}

#[test]
fn parses_and_validates_a_good_config() {
    let config = Config::parse(&config_text(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "extern/occt/9/win64-dev",
    ))
    .unwrap();
    config.validate().unwrap();
    let group = config.group("occt").unwrap();
    assert_eq!(group.repo(), "example/occt-dev");
    assert_eq!(group.tag(), "v9");
    let artifact = group
        .section(HostOs::Windows)
        .unwrap()
        .artifacts()
        .next()
        .unwrap()
        .1;
    assert_eq!(
        artifact.url(),
        "https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip"
    );
    assert_eq!(artifact.dest(), "extern/occt/9/win64-dev");
    assert_eq!(
        artifact.pinned_sha256(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
}

#[test]
fn empty_sha256_means_unpinned_and_still_validates() {
    let config = Config::parse(&config_text("", "extern/x")).unwrap();
    config.validate().unwrap();
    let artifact = config
        .group("occt")
        .unwrap()
        .section(HostOs::Windows)
        .unwrap()
        .artifacts()
        .next()
        .unwrap()
        .1;
    assert_eq!(artifact.pinned_sha256(), None);
}

#[test]
fn url_must_be_a_release_asset_of_the_same_repo_and_tag() {
    let text = config_text(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "extern/x",
    )
    .replace("example/occt-dev/releases", "other/releases");
    let error = Config::parse(&text).unwrap().validate().unwrap_err();
    assert!(error.to_string().contains("same-tag"), "{error}");
}

#[test]
fn sha256_must_be_64_hex_digits_when_pinned() {
    let error = Config::parse(&config_text("abc123", "extern/x"))
        .unwrap()
        .validate()
        .unwrap_err();
    assert!(error.to_string().contains("64 hex digits"), "{error}");
}

#[test]
fn dest_rejects_parent_directory_components() {
    let error = Config::parse(&config_text(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "extern/../evil",
    ))
    .unwrap()
    .validate()
    .unwrap_err();
    assert!(error.to_string().contains("'..'"), "{error}");
}

#[test]
fn dest_rejects_backslashes() {
    // TOML literal string (single quotes) — a backslash cannot appear in
    // a TOML basic string without escaping, which is exactly the trap
    // this rule prevents for Windows-authored dests.
    let text = "[occt]\nrepo = \"example/occt-dev\"\ntag = \"v9\"\n\n[occt.windows.dev]\nurl = \"https://github.com/example/occt-dev/releases/download/v9/occt-dev.zip\"\nsha256 = \"\"\ndest = 'extern\\occt'\n";
    let error = Config::parse(text).unwrap().validate().unwrap_err();
    assert!(error.to_string().contains("forward slashes"), "{error}");
}

#[test]
fn dest_rejects_colon_components() {
    // Rejected on every platform so the committed config means the same
    // thing everywhere. Which check fires is host-dependent — on Windows
    // "C:/extern" parses as an absolute path (drive prefix), elsewhere
    // it is a relative path whose first component carries the colon —
    // so only the rejection itself is host-independent; assert the
    // message of the check that fires on this host.
    let error = Config::parse(&config_text(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "C:/extern",
    ))
    .unwrap()
    .validate()
    .unwrap_err();
    let expected = match HostOs::current().unwrap() {
        HostOs::Windows => "dest must",
        HostOs::Linux | HostOs::MacOs => "contains ':'",
    };
    assert!(error.to_string().contains(expected), "{error}");
}

#[test]
fn unknown_os_section_fails_validation() {
    let text = format!(
        "{}\n[occt.windwos.dev]\nurl = \"https://github.com/example/occt-dev/releases/download/v9/x.zip\"\ndest = \"extern/x\"\n",
        config_text("", "extern/x")
    );
    let error = Config::parse(&text).unwrap().validate().unwrap_err();
    assert!(
        error.to_string().contains("not a known OS section"),
        "{error}"
    );
}

#[test]
fn missing_os_section_is_the_platform_gate() {
    let config = Config::parse(&config_text("", "extern/x")).unwrap();
    assert!(
        config
            .group("occt")
            .unwrap()
            .section(HostOs::Linux)
            .is_none()
    );
}

#[test]
fn top_level_download_location_is_tolerated_and_defaulted() {
    // The key is honored by both consumers (this tool's ensure and the
    // Unity C# provisioner); here it must parse cleanly (not be
    // swallowed into the flattened group map), default to empty when
    // absent, and carry the set value when present.
    let config = Config::parse(&config_text("", "extern/x")).unwrap();
    assert!(config.download_location.is_empty());
    let text = format!(
        "download_location = \"D:/unity-cache\"\n{}",
        config_text("", "extern/x")
    );
    let config = Config::parse(&text).unwrap();
    assert_eq!(config.download_location, "D:/unity-cache");
    config.validate().unwrap();
}

#[test]
fn download_location_empty_resolves_to_memory_mode() {
    let config = Config::parse(&config_text("", "extern/x")).unwrap();
    assert!(config.download_dir().unwrap().is_none());
}

#[test]
fn download_location_relative_is_rejected() {
    let text = format!(
        "download_location = \"staging\"\n{}",
        config_text("", "extern/x")
    );
    let error = Config::parse(&text).unwrap().download_dir().unwrap_err();
    assert!(error.to_string().contains("absolute path"), "{error}");
}

#[test]
fn download_location_missing_directory_is_rejected() {
    // The path must be absolute on this host (a relative path is
    // rejected earlier, with a different message) yet not exist, so the
    // existence check — not the absolute check — produces the failure.
    // Windows needs a drive prefix to be absolute; elsewhere a leading
    // slash does it.
    let missing = match HostOs::current().unwrap() {
        HostOs::Windows => "D:/no-such-dir-7f3d",
        HostOs::Linux | HostOs::MacOs => "/no-such-dir-7f3d",
    };
    let text = format!(
        "download_location = \"{missing}\"\n{}",
        config_text("", "extern/x")
    );
    let error = Config::parse(&text).unwrap().download_dir().unwrap_err();
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn download_location_existing_directory_resolves() {
    let temp = TempDir::new().unwrap();
    let text = format!(
        "download_location = \"{}\"\n{}",
        temp.path().to_string_lossy().replace('\\', "/"),
        config_text("", "extern/x")
    );
    let resolved = Config::parse(&text).unwrap().download_dir().unwrap();
    assert_eq!(resolved, Some(temp.path().to_path_buf()));
}

#[test]
fn repo_root_is_two_levels_above_the_config_directory() {
    let temp = TempDir::new().unwrap();
    let crate_dir = temp.path().join("crates").join("extern-fetch");
    fs::create_dir_all(&crate_dir).unwrap();
    let config_path = crate_dir.join("config.toml");
    fs::write(&config_path, config_text("", "extern/x")).unwrap();
    let root = Config::repo_root(&config_path).unwrap();
    assert_eq!(root, temp.path());
    let config = Config::parse(&config_text("", "extern/x")).unwrap();
    let artifact = config
        .group("occt")
        .unwrap()
        .section(HostOs::Windows)
        .unwrap()
        .artifacts()
        .next()
        .unwrap()
        .1;
    assert_eq!(
        artifact.dest_path(&root),
        temp.path().join("extern").join("x"),
        "dest resolves against the repo root, not the cwd"
    );
    assert_eq!(
        artifact.marker_path(&root),
        temp.path().join("extern").join("x.provenance.toml"),
        "marker is a sibling of the destination directory"
    );
}

#[test]
fn config_path_outside_a_crate_layout_is_rejected() {
    // A config directly at a filesystem root has no two-levels-up parent.
    let error = Config::repo_root(Path::new("/config.toml"));
    assert!(
        error
            .unwrap_err()
            .to_string()
            .contains("two directory levels"),
        "expected the two-levels rule error"
    );
}
