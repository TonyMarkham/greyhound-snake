use crate::export::{ExportOptions, export};

use std::{fs, path::PathBuf, sync::Mutex};

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn options(step: &str, free_root: bool, out: &std::path::Path) -> ExportOptions {
    let root = workspace_root();
    ExportOptions {
        config_path: root.join("config.toml"),
        step_path: root.join(step),
        out_dir: out.to_path_buf(),
        density: 1.0,
        free_root,
        scale: 0.001,
    }
}

/// Each test below runs a full OCCT native load/unload cycle (dlopen of the
/// shim and its libTK* closure, then dlclose on drop). Concurrent cycles
/// interleave OCCT initialization and teardown and segfault (reproduced 3/40
/// parallel runs, 0/15 serial), so the tests serialize on this lock while
/// every other test in the binary keeps running in parallel.
static OCCT_LOAD_SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn given_cart_asy_when_exported_then_counts_masses_and_files_match() {
    let _occt_serial = OCCT_LOAD_SERIAL.lock().unwrap();
    let out = tempfile::tempdir().unwrap();
    let summary = export(&options("assets/cart-asy.step", false, out.path())).unwrap();

    assert_eq!(summary.body_count, 43);
    assert_eq!(summary.mesh_count, 20);
    assert_eq!(summary.geom_count, 39);
    // 39 part occurrences summing 255,450.2 mm3 at 1.0 g/cm3.
    assert!(
        (summary.total_mass_kg - 0.2554502).abs() < 1e-6,
        "{}",
        summary.total_mass_kg
    );

    let text = fs::read_to_string(out.path().join("cart-asy.xml")).unwrap();
    assert!(text.contains("angle=\"radian\""), "{text}");
    assert!(text.contains("meshdir=\"meshes\""), "{text}");
    assert!(text.contains("inertiafromgeom=\"auto\""), "{text}");
    assert!(!text.contains("<freejoint/>"));
    // The cart part's local COM, OCCT (16.9977, 62.9348, -0.2281) mm,
    // identity-mapped and scaled: (0.0169977, 0.0629348, -0.0002281) m.
    // (Unity receives (0.0169977, -0.0002281, 0.0629348) — the (x, z, y)
    // permutation is the cheapest identity-map regression check.)
    assert!(text.contains("0.016997"), "cart COM x missing");
    assert!(text.contains("0.062934"), "cart COM y missing");
    assert!(text.contains("-0.00022"), "cart COM z missing");

    let mesh_files = fs::read_dir(out.path().join("meshes")).unwrap().count();
    assert_eq!(mesh_files, 20);
}

#[test]
fn given_rod_clamp_with_free_root_when_exported_then_the_root_floats() {
    let _occt_serial = OCCT_LOAD_SERIAL.lock().unwrap();
    let out = tempfile::tempdir().unwrap();
    let summary = export(&options("assets/rod-clamp-16mm.stp", true, out.path())).unwrap();

    assert_eq!(summary.body_count, 1);
    assert_eq!(summary.mesh_count, 1);
    assert_eq!(summary.geom_count, 1);
    // 3,182.62 mm3 at 1.0 g/cm3.
    assert!(
        (summary.total_mass_kg - 3.18262e-3).abs() < 1e-7,
        "{}",
        summary.total_mass_kg
    );

    let text = fs::read_to_string(out.path().join("rod-clamp-16mm.xml")).unwrap();
    let freejoint = text.find("<freejoint/>").unwrap();
    let geom = text.find("<geom type=\"mesh\"").unwrap();
    assert!(freejoint < geom, "{text}");
}
