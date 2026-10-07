use crate::{Occt, OcctError};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("OCCT_TEST_FIXTURE_DIR")).join(name)
}

fn load_fixture(name: &str) -> Result<Occt, OcctError> {
    let directory = fixture(name);
    Occt::load(&directory, &directory.join("libgreyhound_occt.so"))
}

fn probe(name: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let full_name = format!("tests::runtime_loading::{name}");

    Ok(Command::new(env::current_exe()?)
        .args([
            "--exact",
            &full_name,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()?)
}

#[test]
fn lifetime_normal_return() -> Result<(), Box<dyn std::error::Error>> {
    let output = probe("probe_normal_return")?;
    assert!(output.status.success());
    let events = String::from_utf8(output.stderr)?;
    assert_eq!(events.matches("fixture:open\n").count(), 1);
    assert_eq!(events.matches("fixture:close\n").count(), 1);
    let opened = events.find("fixture:open\n").ok_or("missing open event")?;
    let closed = events
        .find("fixture:close\n")
        .ok_or("missing close event")?;
    assert!(opened < closed);
    Ok(())
}

#[test]
#[ignore = "subprocess helper; invoked by lifetime_normal_return"]
fn probe_normal_return() -> Result<(), Box<dyn std::error::Error>> {
    let occt = load_fixture("valid")?;
    let doc = occt.open_step(Path::new("fixture.step"))?;
    assert_eq!(doc.info()?.solids, 1);
    Ok(()) // document closes normally before its context drops.
}

#[test]
fn lifetime_exit_without_drop() -> Result<(), Box<dyn std::error::Error>> {
    let output = probe("probe_exit_without_drop")?;
    assert!(output.status.success());
    let events = String::from_utf8(output.stderr)?;
    assert_eq!(events.matches("fixture:open\n").count(), 1);
    assert_eq!(events.matches("fixture:close\n").count(), 0);
    Ok(())
}

#[test]
#[ignore = "subprocess helper; invoked by lifetime_exit_without_drop"]
fn probe_exit_without_drop() -> Result<(), Box<dyn std::error::Error>> {
    let occt = load_fixture("valid")?;
    let _doc = occt.open_step(Path::new("fixture.step"))?;
    std::process::exit(0); // deliberately bypass Rust Drop in this child only.
}

#[test]
fn lifetime_load_failure_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    for name in ["probe_missing_symbol", "probe_bad_abi"] {
        let output = probe(name)?;
        assert!(output.status.success());
        let events = String::from_utf8(output.stderr)?;
        assert_eq!(events.matches("fixture:load\n").count(), 1);
        assert_eq!(events.matches("fixture:open\n").count(), 0);
        // These simple fixtures contain no retaining native TLS destructor.
        // Check fixture cleanup, not a general OCCT unmapping guarantee.
        assert_eq!(events.matches("fixture:unload\n").count(), 1);
    }
    Ok(())
}

#[test]
#[ignore = "subprocess helper; invoked by lifetime_load_failure_cleanup"]
fn probe_missing_symbol() {
    assert!(matches!(
        load_fixture("missing-symbol"),
        Err(OcctError::Symbol { .. })
    ));
}

#[test]
#[ignore = "subprocess helper; invoked by lifetime_load_failure_cleanup"]
fn probe_bad_abi() {
    assert!(matches!(
        load_fixture("bad-abi"),
        Err(OcctError::Abi { .. })
    ));
}

#[test]
fn lifetime_open_failure_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let output = probe("probe_open_failure")?;
    assert!(output.status.success());
    let events = String::from_utf8(output.stderr)?;
    assert_eq!(events.matches("fixture:partial-cleanup\n").count(), 1);
    assert_eq!(events.matches("fixture:open\n").count(), 0);
    assert_eq!(events.matches("fixture:close\n").count(), 0);
    Ok(())
}

#[test]
#[ignore = "subprocess helper; invoked by lifetime_open_failure_cleanup"]
fn probe_open_failure() -> Result<(), Box<dyn std::error::Error>> {
    let occt = load_fixture("open-failure")?;
    assert!(occt.open_step(Path::new("fixture.step")).is_err());
    Ok(())
}

#[test]
fn shim_only_directory() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::copy(
        fixture("valid").join("libgreyhound_occt.so"),
        directory.path().join("libgreyhound_occt.so"),
    )?;
    let result = Occt::load(
        directory.path(),
        &directory.path().join("libgreyhound_occt.so"),
    );
    let Err(error @ OcctError::Load { .. }) = result else {
        return Err("shim-only directory did not produce a Load error".into());
    };
    let message = error.to_string();
    assert!(message.contains(&directory.path().display().to_string()));
    assert!(message.contains("libgreyhound_occt.so"));
    assert!(message.contains("libTKernel.so"));
    assert!(message.contains("missing OCCT library"));
    Ok(())
}
