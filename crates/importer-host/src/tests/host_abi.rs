use crate::{HostMeshCounts, HostSceneCounts, guard::guard, last_error};

use unity_projection::{ProjectionSettings, UnitySubMesh, UnityVertex, project_scene};

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
type SceneCountsFn = unsafe extern "C" fn(*mut c_void, f64, f64, f64, *mut HostSceneCounts) -> i32;
type SceneFillFn = unsafe extern "C" fn(*mut c_void, *mut u32, *mut [f32; 12], *mut u8) -> i32;
type ColorFillFn = unsafe extern "C" fn(*mut c_void, *mut [f32; 4]) -> i32;
type MeshCountsFn = unsafe extern "C" fn(*mut c_void, u32, *mut HostMeshCounts) -> i32;
type MeshFillFn = unsafe extern "C" fn(
    *mut c_void,
    u32,
    *mut UnityVertex,
    *mut u32,
    *mut UnitySubMesh,
    *mut u32,
) -> i32;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn host_artifact() -> PathBuf {
    // The workspace shares one target tree; plain `cargo test` builds the
    // cdylib into the debug profile directory.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libimporter_host.so")
}

fn occt_dirs(root: &Path) -> (PathBuf, PathBuf) {
    (
        root.join("dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib"),
        root.join(
            "dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so",
        ),
    )
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
fn given_real_occt_when_importing_an_assembly_then_the_scene_matches_the_direct_pipeline() {
    // given
    let root = workspace_root();
    let (occt_dir, shim_path) = occt_dirs(&root);
    let asset = root.join("assets/cart-asy.step");
    let library = unsafe { Library::new(host_artifact()) }.unwrap();

    unsafe {
        let version: Symbol<VersionFn> = library.get(b"greyhound_host_version\0").unwrap();
        let last_error: Symbol<LastErrorFn> = library.get(b"greyhound_host_last_error\0").unwrap();
        let host_new: Symbol<HostNewFn> = library.get(b"greyhound_host_new\0").unwrap();
        let host_free: Symbol<HostFreeFn> = library.get(b"greyhound_host_free\0").unwrap();
        let open_step: Symbol<OpenStepFn> = library.get(b"greyhound_host_open_step\0").unwrap();
        let close_step: Symbol<CloseStepFn> = library.get(b"greyhound_host_close_step\0").unwrap();
        let scene_counts: Symbol<SceneCountsFn> =
            library.get(b"greyhound_host_scene_counts\0").unwrap();
        let scene_fill: Symbol<SceneFillFn> = library.get(b"greyhound_host_scene_fill\0").unwrap();
        let color_fill: Symbol<ColorFillFn> = library.get(b"greyhound_host_color_fill\0").unwrap();
        let mesh_counts: Symbol<MeshCountsFn> =
            library.get(b"greyhound_host_mesh_counts\0").unwrap();
        let mesh_fill: Symbol<MeshFillFn> = library.get(b"greyhound_host_mesh_fill\0").unwrap();

        // when
        assert_eq!(version(), 4);
        assert!(last_error().is_null());

        let host = host_new(c_string(&occt_dir).as_ptr(), c_string(&shim_path).as_ptr());
        assert!(!host.is_null(), "{}", last_error_text(&last_error));

        let doc = open_step(host, c_string(&asset).as_ptr());
        assert!(!doc.is_null(), "{}", last_error_text(&last_error));

        let mut counts = HostSceneCounts {
            node_count: 0,
            mesh_count: 0,
            color_count: 0,
            name_bytes: 0,
        };
        assert_eq!(scene_counts(doc, 0.01, 0.5, 0.001, &mut counts), 0);

        let mut nodes = vec![0u32; usize::try_from(counts.node_count).unwrap() * 4];
        let mut transforms = vec![[0.0f32; 12]; usize::try_from(counts.node_count).unwrap()];
        let mut names = vec![0u8; usize::try_from(counts.name_bytes).unwrap()];
        assert_eq!(
            scene_fill(
                doc,
                nodes.as_mut_ptr(),
                transforms.as_mut_ptr(),
                names.as_mut_ptr()
            ),
            0
        );

        let mut colors = vec![[0.0f32; 4]; usize::try_from(counts.color_count).unwrap()];
        assert_eq!(color_fill(doc, colors.as_mut_ptr()), 0);

        let mut mesh_zero = HostMeshCounts {
            vertex_count: 0,
            index_count: 0,
            submesh_count: 0,
            color_count: 0,
            bounds: unity_projection::UnityBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        };
        assert_eq!(mesh_counts(doc, 0, &mut mesh_zero), 0);
        let mut verts = vec![
            UnityVertex {
                position: [0.0; 3],
                normal: [0.0; 3]
            };
            mesh_zero.vertex_count.try_into().unwrap()
        ];
        let mut indices = vec![0u32; mesh_zero.index_count.try_into().unwrap()];
        let mut submeshes = vec![
            UnitySubMesh {
                index_start: 0,
                index_count: 0,
                first_vertex: 0,
                vertex_count: 0
            };
            mesh_zero.submesh_count.try_into().unwrap()
        ];
        let mut submesh_colors = vec![0u32; mesh_zero.submesh_count.try_into().unwrap()];
        assert_eq!(
            mesh_fill(
                doc,
                0,
                verts.as_mut_ptr(),
                indices.as_mut_ptr(),
                submeshes.as_mut_ptr(),
                submesh_colors.as_mut_ptr()
            ),
            0
        );

        close_step(doc);
        host_free(host);

        // then: the forest shape of the cart assembly
        assert_eq!(counts.node_count, 43);
        assert_eq!(counts.mesh_count, 20);
        assert_eq!(counts.color_count, 15);
        assert!(counts.name_bytes > 0);

        // then: the root is a geometry-free parent node
        assert_eq!(nodes[0], u32::MAX);
        assert_eq!(nodes[1], u32::MAX);

        // then: every parent precedes its child (pre-order)
        for (index, fields) in nodes.as_chunks::<4>().0.iter().enumerate() {
            assert!(
                fields[0] == u32::MAX || u64::from(fields[0]) < index as u64,
                "node {index} parent {} is not a predecessor",
                fields[0]
            );
        }

        // then: the two Pillow Block instances (distinct names, one product)
        // share one mesh (dedup across instances)
        let named = |prefix: &str| -> Vec<usize> {
            nodes
                .as_chunks::<4>()
                .0
                .iter()
                .enumerate()
                .filter_map(|(index, fields)| {
                    let offset = fields[2] as usize;
                    let end = offset + fields[3] as usize;
                    let name = std::str::from_utf8(&names[offset..end]).unwrap();
                    name.starts_with(prefix).then_some(index)
                })
                .collect()
        };
        let pillows = named("Pillow Block");
        assert_eq!(pillows.len(), 2);
        assert_eq!(nodes[4 * pillows[0] + 1], nodes[4 * pillows[1] + 1]);
        assert!(nodes[4 * pillows[0] + 1] != u32::MAX);

        // then: the placed transform is Unity-space with the mm scale baked
        // into the translation; Pillow Block sits at OCCT (17, 18, -19)
        let pillow = transforms[pillows[0]];
        assert!((pillow[3] - 0.017).abs() < 1e-6, "{pillow:?}");
        assert!((pillow[7] - -0.019).abs() < 1e-6, "{pillow:?}");
        assert!((pillow[11] - 0.018).abs() < 1e-6, "{pillow:?}");
        assert!(
            (pillow[0] * pillow[0] + pillow[1] * pillow[1] + pillow[2] * pillow[2] - 1.0).abs()
                < 1e-5
        );

        // then: the direct pipeline agrees with the host buffers
        let direct_occt = occt_sys::Occt::load(&occt_dir, &shim_path).unwrap();
        let direct_doc = direct_occt.open_step(&asset).unwrap();
        let scene = direct_doc.scene(0.01, 0.5).unwrap();
        let projected = project_scene(
            scene.forest(),
            scene.meshes(),
            ProjectionSettings { scale: 0.001 },
        )
        .unwrap();

        let projected_fields: Vec<[u32; 4]> = projected
            .nodes()
            .iter()
            .map(|node| [node.parent, node.mesh, node.name_offset, node.name_length])
            .collect();
        for (fields, expected) in nodes.as_chunks::<4>().0.iter().zip(projected_fields) {
            assert_eq!(*fields, expected);
        }
        assert_eq!(
            transforms,
            projected
                .nodes()
                .iter()
                .map(|node| node.transform)
                .collect::<Vec<_>>()
        );
        assert_eq!(names, projected.names());
        assert_eq!(colors, projected.meshes()[0].colors());
        assert_eq!(verts, projected.meshes()[0].vertices());
        assert_eq!(indices, projected.meshes()[0].indices());
        assert_eq!(submeshes, projected.meshes()[0].submeshes());
        assert_eq!(submesh_colors, projected.meshes()[0].submesh_colors());
        assert_eq!(mesh_zero.color_count, 15);
    }
}
#[test]
fn given_host_abi_when_calls_fail_then_status_and_error_report_the_cause() {
    // given
    let root = workspace_root();
    let (occt_dir, shim_path) = occt_dirs(&root);
    let asset = root.join("assets/cart-asy.step");
    let library = unsafe { Library::new(host_artifact()) }.unwrap();

    unsafe {
        let last_error: Symbol<LastErrorFn> = library.get(b"greyhound_host_last_error\0").unwrap();
        let host_new: Symbol<HostNewFn> = library.get(b"greyhound_host_new\0").unwrap();
        let host_free: Symbol<HostFreeFn> = library.get(b"greyhound_host_free\0").unwrap();
        let open_step: Symbol<OpenStepFn> = library.get(b"greyhound_host_open_step\0").unwrap();
        let close_step: Symbol<CloseStepFn> = library.get(b"greyhound_host_close_step\0").unwrap();
        let scene_counts: Symbol<SceneCountsFn> =
            library.get(b"greyhound_host_scene_counts\0").unwrap();
        let color_fill: Symbol<ColorFillFn> = library.get(b"greyhound_host_color_fill\0").unwrap();
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
        assert_eq!(scene_counts(doc, 0.01, 0.5, 0.001, std::ptr::null_mut()), 1);
        assert!(last_error_text(&last_error).contains("output pointer is null"));

        // when / then: mesh fill before scene counts
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
        let mut submesh_colors = vec![0u32; 1];
        assert_eq!(
            mesh_fill(
                doc,
                0,
                verts.as_mut_ptr(),
                indices.as_mut_ptr(),
                submeshes.as_mut_ptr(),
                submesh_colors.as_mut_ptr()
            ),
            1
        );
        assert!(last_error_text(&last_error).contains("call greyhound_host_scene_counts first"));

        // when / then: mesh index beyond the scene meshes
        let mut counts = HostSceneCounts {
            node_count: 0,
            mesh_count: 0,
            color_count: 0,
            name_bytes: 0,
        };
        assert_eq!(scene_counts(doc, 0.01, 0.5, 0.001, &mut counts), 0);
        let mut mesh_counts_out = HostMeshCounts {
            vertex_count: 0,
            index_count: 0,
            submesh_count: 0,
            color_count: 0,
            bounds: unity_projection::UnityBounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
        };
        assert_eq!(mesh_counts(doc, counts.mesh_count, &mut mesh_counts_out), 1);
        assert!(last_error_text(&last_error).contains("beyond the"));

        // when / then: color fill needs a projected scene
        let mut one_color = [[0.0f32; 4]];
        assert_eq!(color_fill(doc, one_color.as_mut_ptr()), 0);

        close_step(doc);
        host_free(host);
    }
}
