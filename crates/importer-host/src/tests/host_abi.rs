use crate::{HostMeshCounts, guard::guard, last_error};

use occt_sys::Occt;
use unity_projection::{
    OcctBounds, ProjectionSettings, UnityBounds, UnitySubMesh, UnityVertex, project,
};

use libloading::{Library, Symbol};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

type VersionFn = unsafe extern "C" fn() -> u32;
type LastErrorFn = unsafe extern "C" fn() -> *const c_char;
type HostNewFn = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
type HostFreeFn = unsafe extern "C" fn(*mut c_void);
type OpenStepFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
type CloseStepFn = unsafe extern "C" fn(*mut c_void);
type MeshCountsFn = unsafe extern "C" fn(*mut c_void, f64, f64, f64, *mut HostMeshCounts) -> i32;
type MeshFillFn =
    unsafe extern "C" fn(*mut c_void, *mut UnityVertex, *mut u32, *mut UnitySubMesh) -> i32;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn host_artifact() -> PathBuf {
    // The workspace shares one target tree; plain `cargo test` builds the
    // cdylib into the debug profile directory.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libimporter_host.so")
}

fn c_string(path: &Path) -> CString {
    CString::new(path.as_os_str().as_bytes()).unwrap()
}

fn last_error_text(last_error: &Symbol<LastErrorFn>) -> String {
    let pointer = unsafe { last_error() };
    assert!(!pointer.is_null(), "last_error returned null");
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn given_panicking_body_when_guarded_then_fallback_is_returned_and_error_is_set() {
    // given
    last_error::clear_error();

    // when
    let value = guard(7u32, || panic!("boom"));

    // then
    assert_eq!(value, 7);
    let message = unsafe { CStr::from_ptr(last_error::last_error()) }.to_string_lossy();
    assert!(message.contains("boom"), "{message}");
}

#[test]
fn given_cleared_error_when_queried_then_pointer_is_null() {
    // given
    last_error::clear_error();

    // when
    let pointer = last_error::last_error();

    // then
    assert!(pointer.is_null());
}

#[test]
fn given_real_occt_when_importing_through_the_host_abi_then_buffers_match_the_direct_pipeline() {
    // given
    let root = workspace_root();
    let occt_dir = root.join("dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib");
    let shim_path = root
        .join("dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so");
    let asset = root.join("assets/rod-clamp-16mm.stp");
    let library = unsafe { Library::new(host_artifact()) }.unwrap();

    unsafe {
        let version: Symbol<VersionFn> = library.get(b"greyhound_host_version\0").unwrap();
        let last_error: Symbol<LastErrorFn> = library.get(b"greyhound_host_last_error\0").unwrap();
        let host_new: Symbol<HostNewFn> = library.get(b"greyhound_host_new\0").unwrap();
        let host_free: Symbol<HostFreeFn> = library.get(b"greyhound_host_free\0").unwrap();
        let open_step: Symbol<OpenStepFn> = library.get(b"greyhound_host_open_step\0").unwrap();
        let close_step: Symbol<CloseStepFn> = library.get(b"greyhound_host_close_step\0").unwrap();
        let mesh_counts: Symbol<MeshCountsFn> =
            library.get(b"greyhound_host_mesh_counts\0").unwrap();
        let mesh_fill: Symbol<MeshFillFn> = library.get(b"greyhound_host_mesh_fill\0").unwrap();

        // when
        assert_eq!(version(), 1);
        assert!(last_error().is_null());

        let host = host_new(c_string(&occt_dir).as_ptr(), c_string(&shim_path).as_ptr());
        assert!(!host.is_null(), "{}", last_error_text(&last_error));

        let doc = open_step(host, c_string(&asset).as_ptr());
        assert!(!doc.is_null(), "{}", last_error_text(&last_error));

        let mut counts = HostMeshCounts {
            vertex_count: 0,
            index_count: 0,
            submesh_count: 0,
            bounds: UnityBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        };
        assert_eq!(mesh_counts(doc, 0.01, 0.5, 0.001, &mut counts), 0);

        let mut verts = vec![
            UnityVertex {
                position: [0.0; 3],
                normal: [0.0; 3]
            };
            counts.vertex_count.try_into().unwrap()
        ];
        let mut indices = vec![0u32; counts.index_count.try_into().unwrap()];
        let mut submeshes = vec![
            UnitySubMesh {
                index_start: 0,
                index_count: 0,
                first_vertex: 0,
                vertex_count: 0
            };
            counts.submesh_count.try_into().unwrap()
        ];
        assert_eq!(
            mesh_fill(
                doc,
                verts.as_mut_ptr(),
                indices.as_mut_ptr(),
                submeshes.as_mut_ptr()
            ),
            0
        );

        close_step(doc);
        host_free(host);

        // then
        assert_eq!(counts.vertex_count, 1744);
        assert_eq!(counts.index_count, 5580);
        assert_eq!(counts.submesh_count, 26);
        assert!((counts.bounds.min[0] - -0.010).abs() < 1e-6);
        assert!(counts.bounds.min[1] < 0.0 && counts.bounds.min[1] > -0.001);
        assert!((counts.bounds.min[2] - -0.025).abs() < 1e-6);
        assert!((counts.bounds.max[0] - 0.010).abs() < 1e-6);
        assert!((counts.bounds.max[1] - 0.011).abs() < 1e-6);
        assert!((counts.bounds.max[2] - 0.025).abs() < 1e-6);

        let direct = Occt::load(&occt_dir, &shim_path).unwrap();
        let direct_doc = direct.open_step(&asset).unwrap();
        let info = direct_doc.info().unwrap();
        let mesh = direct_doc.mesh(0.01, 0.5).unwrap();
        let projected = project(
            &mesh,
            OcctBounds {
                min: info.bbox.min,
                max: info.bbox.max,
            },
            ProjectionSettings { scale: 0.001 },
        )
        .unwrap();

        assert_eq!(verts, projected.vertices());
        assert_eq!(indices, projected.indices());
        assert_eq!(submeshes, projected.submeshes());
    }
}

#[test]
fn given_host_abi_when_calls_fail_then_status_and_error_report_the_cause() {
    // given
    let root = workspace_root();
    let occt_dir = root.join("dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib");
    let shim_path = root
        .join("dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so");
    let asset = root.join("assets/rod-clamp-16mm.stp");
    let library = unsafe { Library::new(host_artifact()) }.unwrap();

    unsafe {
        let last_error: Symbol<LastErrorFn> = library.get(b"greyhound_host_last_error\0").unwrap();
        let host_new: Symbol<HostNewFn> = library.get(b"greyhound_host_new\0").unwrap();
        let host_free: Symbol<HostFreeFn> = library.get(b"greyhound_host_free\0").unwrap();
        let open_step: Symbol<OpenStepFn> = library.get(b"greyhound_host_open_step\0").unwrap();
        let close_step: Symbol<CloseStepFn> = library.get(b"greyhound_host_close_step\0").unwrap();
        let mesh_counts: Symbol<MeshCountsFn> =
            library.get(b"greyhound_host_mesh_counts\0").unwrap();
        let mesh_fill: Symbol<MeshFillFn> = library.get(b"greyhound_host_mesh_fill\0").unwrap();

        // when / then: null path argument
        assert!(host_new(std::ptr::null(), c_string(&shim_path).as_ptr()).is_null());
        assert!(last_error_text(&last_error).contains("OCCT directory is null"));

        // when / then: existing directory without an OCCT installation
        let bare_dir = root.join("assets");
        assert!(host_new(c_string(&bare_dir).as_ptr(), c_string(&shim_path).as_ptr()).is_null());
        assert!(last_error_text(&last_error).contains("missing OCCT library"));

        // when / then: null counts output pointer
        let host = host_new(c_string(&occt_dir).as_ptr(), c_string(&shim_path).as_ptr());
        assert!(!host.is_null(), "{}", last_error_text(&last_error));
        let doc = open_step(host, c_string(&asset).as_ptr());
        assert!(!doc.is_null(), "{}", last_error_text(&last_error));
        assert_eq!(mesh_counts(doc, 0.01, 0.5, 0.001, std::ptr::null_mut()), 1);
        assert!(last_error_text(&last_error).contains("output pointer is null"));

        // when / then: fill before counts
        let mut verts = vec![UnityVertex {
            position: [0.0; 3],
            normal: [0.0; 3],
        }];
        let mut indices = vec![0u32; 3];
        let mut submeshes = vec![UnitySubMesh {
            index_start: 0,
            index_count: 0,
            first_vertex: 0,
            vertex_count: 0,
        }];
        assert_eq!(
            mesh_fill(
                doc,
                verts.as_mut_ptr(),
                indices.as_mut_ptr(),
                submeshes.as_mut_ptr()
            ),
            1
        );
        assert!(last_error_text(&last_error).contains("call greyhound_host_mesh_counts first"));

        close_step(doc);
        host_free(host);
    }
}
