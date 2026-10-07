use crate::Occt;
use std::path::Path;

#[test]
fn lifetime_context_dropped_first() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Path::new(env!("OCCT_TEST_FIXTURE_DIR")).join("valid");
    let shim = directory.join("libgreyhound_occt.so");
    let occt = Occt::load(&directory, &shim)?;
    let weak = occt.weak_api();
    let doc = occt.open_step(Path::new("fixture.step"))?;
    drop(occt);
    assert!(weak.upgrade().is_some());
    assert_eq!(doc.info()?.solids, 1);
    drop(doc);
    assert!(weak.upgrade().is_none());
    Ok(())
}

#[test]
fn lifetime_doc_dropped_first() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Path::new(env!("OCCT_TEST_FIXTURE_DIR")).join("valid");
    let shim = directory.join("libgreyhound_occt.so");
    let occt = Occt::load(&directory, &shim)?;
    let weak = occt.weak_api();
    let doc = occt.open_step(Path::new("fixture.step"))?;
    drop(doc);
    assert!(weak.upgrade().is_some());
    assert!((occt.box_volume(10.0, 20.0, 30.0)? - 6000.0).abs() < 1e-8);
    drop(occt);
    assert!(weak.upgrade().is_none());
    Ok(())
}
