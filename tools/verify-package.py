#!/usr/bin/env python3
"""Verify the assembled native package from a copied layout.

Replays the exact C# consumption flow of the importer-host ABI against a
temporary copy of package/ so path independence is actually exercised:
dlopen host -> version probe -> greyhound_host_new(occt_dir, shim) ->
open_step -> scene_counts -> scene_fill -> color_fill -> per-mesh
mesh_counts -> mesh_fill.

The expected numbers are the reference asset results recorded in
occt-to-unity.md.
"""

import argparse
import ctypes
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

PACKAGE_NAME = "com.greyhound.step"

EXPECTED_NODE_COUNT = 43
EXPECTED_MESH_COUNT = 20
EXPECTED_COLOR_COUNT = 15
# OCCT (17, 18, -19) permuted to Unity (x, z, y) and scaled to meters.
EXPECTED_PILLOW_TRANSLATION = (0.017, -0.019, 0.018)
EXPECTED_TOTAL_VERTICES = 69519
EXPECTED_TOTAL_TRIANGLES = 91512

DEFLECTION = 0.01
ANGLE_RAD = 0.5
SCALE = 0.001

HOST_ABI_VERSION = 4

NO_INDEX = 0xFFFFFFFF


class UnityBounds(ctypes.Structure):
    _fields_ = [("min", ctypes.c_float * 3), ("max", ctypes.c_float * 3)]


class HostSceneCounts(ctypes.Structure):
    _fields_ = [
        ("node_count", ctypes.c_uint32),
        ("mesh_count", ctypes.c_uint32),
        ("color_count", ctypes.c_uint32),
        ("name_bytes", ctypes.c_uint32),
    ]


class HostMeshProperties(ctypes.Structure):
    _fields_ = [
        ("volume_mm3", ctypes.c_float),
        ("file_density", ctypes.c_float),
        ("centre_of_gravity", ctypes.c_float * 3),
        ("gyration_radii", ctypes.c_float * 3),
        ("principal_axes", ctypes.c_float * 9),
    ]


class HostMeshCounts(ctypes.Structure):
    _fields_ = [
        ("vertex_count", ctypes.c_uint32),
        ("index_count", ctypes.c_uint32),
        ("submesh_count", ctypes.c_uint32),
        ("color_count", ctypes.c_uint32),
        ("bounds", UnityBounds),
    ]


class UnityVertex(ctypes.Structure):
    _fields_ = [("position", ctypes.c_float * 3), ("normal", ctypes.c_float * 3)]


class UnitySubMesh(ctypes.Structure):
    _fields_ = [
        ("index_start", ctypes.c_uint32),
        ("index_count", ctypes.c_uint32),
        ("first_vertex", ctypes.c_uint32),
        ("vertex_count", ctypes.c_uint32),
    ]


def fail(message):
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, default=Path("dist/package"))
    parser.add_argument("--asset", type=Path, default=Path("assets/cart-asy.step"))
    args = parser.parse_args()

    if not (args.package / PACKAGE_NAME / "Runtime" / "Plugins" / "x86_64" / "libimporter_host.so").is_file():
        fail(f"{args.package} is not an assembled package; run 'just assemble-package'")

    with tempfile.TemporaryDirectory(prefix="greyhound-package-") as copy:
        package = Path(copy) / "package"
        shutil.copytree(args.package, package, symlinks=False)
        verify(package, args.asset.resolve())
    print("PASS")


def verify(package, asset):
    root = package / PACKAGE_NAME
    with (root / "package.json").open("rb") as handle:
        manifest = json.load(handle)
    if manifest.get("name") != PACKAGE_NAME:
        fail(f"package.json declares name {manifest.get('name')!r}, expected {PACKAGE_NAME!r}")
    if not (root / "Third Party Notices.md").is_file():
        fail("Third Party Notices.md is missing from the package root")

    library_dir = root / "Runtime" / "Plugins" / "occt" / "x86_64" / "lib"
    lib = ctypes.CDLL(str(root / "Runtime" / "Plugins" / "x86_64" / "libimporter_host.so"))

    lib.greyhound_host_version.restype = ctypes.c_uint32
    version = lib.greyhound_host_version()
    if version != HOST_ABI_VERSION:
        fail(f"host ABI version {version}, expected {HOST_ABI_VERSION}")

    lib.greyhound_host_last_error.restype = ctypes.c_char_p
    lib.greyhound_host_new.restype = ctypes.c_void_p
    lib.greyhound_host_new.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.greyhound_host_open_step.restype = ctypes.c_void_p
    lib.greyhound_host_open_step.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.greyhound_host_scene_counts.restype = ctypes.c_int32
    lib.greyhound_host_scene_counts.argtypes = [
        ctypes.c_void_p,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.POINTER(HostSceneCounts),
    ]
    lib.greyhound_host_scene_fill.restype = ctypes.c_int32
    lib.greyhound_host_scene_fill.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(ctypes.c_float),
        ctypes.POINTER(ctypes.c_uint8),
    ]
    lib.greyhound_host_color_fill.restype = ctypes.c_int32
    lib.greyhound_host_color_fill.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_float),
    ]
    lib.greyhound_host_mesh_properties.restype = ctypes.c_int32
    lib.greyhound_host_mesh_properties.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.POINTER(HostMeshProperties),
    ]
    lib.greyhound_host_mesh_counts.restype = ctypes.c_int32
    lib.greyhound_host_mesh_counts.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.POINTER(HostMeshCounts),
    ]
    lib.greyhound_host_mesh_fill.restype = ctypes.c_int32
    lib.greyhound_host_mesh_fill.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint32,
        ctypes.POINTER(UnityVertex),
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(UnitySubMesh),
        ctypes.POINTER(ctypes.c_uint32),
    ]
    lib.greyhound_host_close_step.argtypes = [ctypes.c_void_p]
    lib.greyhound_host_free.argtypes = [ctypes.c_void_p]

    host = lib.greyhound_host_new(
        os.fsencode(library_dir),
        os.fsencode(root / "Runtime" / "Plugins" / "shim" / "x86_64" / "libgreyhound_occt.so"),
    )
    if not host:
        fail(f"greyhound_host_new: {lib.greyhound_host_last_error().decode()}")

    doc = lib.greyhound_host_open_step(host, os.fsencode(asset))
    if not doc:
        fail(f"greyhound_host_open_step: {lib.greyhound_host_last_error().decode()}")

    scene = HostSceneCounts()
    status = lib.greyhound_host_scene_counts(doc, DEFLECTION, ANGLE_RAD, SCALE, ctypes.byref(scene))
    if status != 0:
        fail(f"greyhound_host_scene_counts: {lib.greyhound_host_last_error().decode()}")
    if (
        scene.node_count != EXPECTED_NODE_COUNT
        or scene.mesh_count != EXPECTED_MESH_COUNT
        or scene.color_count != EXPECTED_COLOR_COUNT
        or scene.name_bytes == 0
    ):
        fail(
            f"unexpected scene counts: {scene.node_count} nodes, "
            f"{scene.mesh_count} meshes, {scene.color_count} colors, "
            f"{scene.name_bytes} name bytes"
        )
    print(
        f"scene: {scene.node_count} nodes, {scene.mesh_count} meshes, "
        f"{scene.color_count} colors, {scene.name_bytes} name bytes"
    )

    nodes = (ctypes.c_uint32 * (scene.node_count * 4))()
    transforms = (ctypes.c_float * (scene.node_count * 12))()
    names = (ctypes.c_uint8 * scene.name_bytes)()
    status = lib.greyhound_host_scene_fill(doc, nodes, transforms, names)
    if status != 0:
        fail(f"greyhound_host_scene_fill: {lib.greyhound_host_last_error().decode()}")

    # Pre-order invariants: roots carry the sentinels; every parent precedes
    # its child.
    roots = 0
    for index in range(scene.node_count):
        parent = nodes[index * 4]
        mesh = nodes[index * 4 + 1]
        if parent == NO_INDEX:
            roots += 1
        elif parent >= index:
            fail(f"node {index} parent {parent} is not a predecessor")
        if mesh != NO_INDEX and mesh >= scene.mesh_count:
            fail(f"node {index} references mesh {mesh} of {scene.mesh_count}")
    if roots != 1:
        fail(f"expected exactly 1 scene root, found {roots}")

    def node_name(index):
        offset = nodes[index * 4 + 2]
        length = nodes[index * 4 + 3]
        return bytes(names[offset : offset + length]).decode("utf-8")

    pillow_nodes = [i for i in range(scene.node_count) if node_name(i).startswith("Pillow Block")]
    if len(pillow_nodes) != 2:
        fail(f"expected 2 Pillow Block instances, found {len(pillow_nodes)}")
    pillow_meshes = {nodes[i * 4 + 1] for i in pillow_nodes}
    if len(pillow_meshes) != 1:
        fail("the two Pillow Block instances do not share one mesh")
    pillow_transform = transforms[pillow_nodes[0] * 12 : pillow_nodes[0] * 12 + 12]
    translation = tuple(pillow_transform[column * 4 + 3] for column in range(3))
    if any(abs(a - e) > 1e-6 for a, e in zip(translation, EXPECTED_PILLOW_TRANSLATION)):
        fail(
            f"unexpected Pillow Block translation {translation}, "
            f"expected {EXPECTED_PILLOW_TRANSLATION}"
        )
    rotation_norm = (
        pillow_transform[0] ** 2 + pillow_transform[1] ** 2 + pillow_transform[2] ** 2
    ) ** 0.5
    if abs(rotation_norm - 1.0) > 1e-5:
        fail(f"the Pillow Block rotation row is not unit length: {rotation_norm}")
    print(
        f"nodes: root mesh={nodes[1] == NO_INDEX}; Pillow Block instances "
        f"{pillow_nodes} share mesh {nodes[pillow_nodes[0] * 4 + 1]}, "
        f"translation ({translation[0]:.3f} {translation[1]:.3f} {translation[2]:.3f})"
    )

    # Nested composition guard: a bearing sub-part's WORLD transform must
    # equal the product of its ancestors' LOCAL transforms (root -> bearing
    # sub-assembly -> part). Double-applied parent transforms - the bug this
    # bite shipped once - show up as a doubled translation here.
    def node_transform(index):
        # row-major 3x4: rows 0..2, columns 0..3 (column 3 = translation)
        base = index * 12
        return [
            [transforms[base + row * 4 + col] for col in range(4)]
            for row in range(3)
        ]

    def compose(parent, child):
        # world = parent @ child for row-major 3x4 affine matrices
        result = [[0.0] * 4 for _ in range(3)]
        for row in range(3):
            for col in range(3):
                result[row][col] = sum(
                    parent[row][k] * child[k][col] for k in range(3)
                )
            result[row][3] = (
                sum(parent[row][k] * child[k][3] for k in range(3))
                + parent[row][3]
            )
        return result

    def world_of(index):
        chain = []
        current = index
        while current != NO_INDEX:
            chain.append(node_transform(current))
            current = nodes[current * 4]
        world = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]
        for local in reversed(chain):
            world = compose(world, local)
        return world

    bearing010 = next(
        (i for i in range(scene.node_count) if node_name(i) == "5972K91_Steel Ball Bearing010"),
        None,
    )
    if bearing010 is None:
        fail("no node named 5972K91_Steel Ball Bearing010")
    first_child = next(
        (i for i in range(scene.node_count) if nodes[i * 4] == bearing010),
        None,
    )
    if first_child is None:
        fail("the bearing sub-assembly has no children")
    world = world_of(first_child)
    # Measured ground truth for the bearing010 instance placement: OCCT
    # translation (17, -15, 13) with the placement's axis rotation, permuted
    # to Unity space ((x, z, y) conjugation) and scaled. A double-applied
    # parent transform would square the rotation and shift the translation.
    expected_world = (0.017, 0.013, -0.015)
    expected_rotation = [
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ]
    actual_world = tuple(world[row][3] for row in range(3))
    if any(abs(a - e) > 1e-6 for a, e in zip(actual_world, expected_world)):
        fail(
            f"unexpected nested world translation {actual_world}, "
            f"expected {expected_world} (parent transform applied twice?)"
        )
    for row in range(3):
        for col in range(3):
            if abs(world[row][col] - expected_rotation[row][col]) > 1e-5:
                fail(
                    f"unexpected nested world rotation at ({row}, {col}): "
                    f"{world[row][col]} (parent transform applied twice?)"
                )
    print(
        f"nested: bearing part world translation "
        f"({actual_world[0]:.3f} {actual_world[1]:.3f} {actual_world[2]:.3f}), "
        f"rotation ok"
    )

    colors = (ctypes.c_float * (scene.color_count * 4))()
    status = lib.greyhound_host_color_fill(doc, colors)
    if status != 0:
        fail(f"greyhound_host_color_fill: {lib.greyhound_host_last_error().decode()}")

    total_vertices = 0
    total_triangles = 0
    for mesh_index in range(scene.mesh_count):
        counts = HostMeshCounts()
        status = lib.greyhound_host_mesh_counts(doc, mesh_index, ctypes.byref(counts))
        if status != 0:
            fail(f"greyhound_host_mesh_counts({mesh_index}): {lib.greyhound_host_last_error().decode()}")
        if counts.color_count != scene.color_count:
            fail(f"mesh {mesh_index} reports {counts.color_count} colors of {scene.color_count}")

        verts = (UnityVertex * counts.vertex_count)()
        indices = (ctypes.c_uint32 * counts.index_count)()
        submeshes = (UnitySubMesh * counts.submesh_count)()
        submesh_colors = (ctypes.c_uint32 * counts.submesh_count)()
        status = lib.greyhound_host_mesh_fill(
            doc, mesh_index, verts, indices, submeshes, submesh_colors
        )
        if status != 0:
            fail(f"greyhound_host_mesh_fill({mesh_index}): {lib.greyhound_host_last_error().decode()}")

        covered = sum(submesh.index_count for submesh in submeshes)
        if covered != counts.index_count:
            fail(f"mesh {mesh_index} submesh ranges cover {covered} of {counts.index_count} indices")
        for color_index in submesh_colors:
            if color_index >= scene.color_count:
                fail(f"mesh {mesh_index} submesh references color {color_index} of {scene.color_count}")
        first = verts[0]
        magnitude = sum(component * component for component in first.normal) ** 0.5
        if abs(magnitude - 1.0) > 1e-3:
            fail(f"mesh {mesh_index} first vertex normal is not unit length: {magnitude}")

        total_vertices += counts.vertex_count
        total_triangles += counts.index_count // 3
        print(
            f"mesh {mesh_index}: {counts.vertex_count} verts, "
            f"{counts.index_count // 3} tris, {counts.submesh_count} submeshes, bounds "
            f"[{counts.bounds.min[0]:.4f} {counts.bounds.min[1]:.4f} "
            f"{counts.bounds.min[2]:.4f}] .. "
            f"[{counts.bounds.max[0]:.4f} {counts.bounds.max[1]:.4f} "
            f"{counts.bounds.max[2]:.4f}]"
        )

    if (
        total_vertices != EXPECTED_TOTAL_VERTICES
        or total_triangles != EXPECTED_TOTAL_TRIANGLES
    ):
        fail(
            f"unexpected mesh totals: {total_vertices} verts, {total_triangles} tris, "
            f"expected {EXPECTED_TOTAL_VERTICES} verts, {EXPECTED_TOTAL_TRIANGLES} tris"
        )

    # Exact BRep mass properties per unique mesh: orthonormal principal
    # axes, positive matched moments (mass-free gyration radii), the
    # measured cart volume and local centre of gravity, and no file
    # density (this file carries no materials).
    for mesh_index in range(scene.mesh_count):
        props = HostMeshProperties()
        status = lib.greyhound_host_mesh_properties(doc, mesh_index, ctypes.byref(props))
        if status != 0:
            fail(
                f"greyhound_host_mesh_properties({mesh_index}): "
                f"{lib.greyhound_host_last_error().decode()}"
            )
        if props.volume_mm3 <= 0 or props.file_density != 0.0:
            fail(f"mesh {mesh_index}: volume {props.volume_mm3}, file density {props.file_density}")
        axes = [props.principal_axes[i * 3 : (i + 1) * 3] for i in range(3)]
        for row in range(3):
            norm = sum(v * v for v in axes[row]) ** 0.5
            if abs(norm - 1.0) > 1e-4:
                fail(f"mesh {mesh_index} principal axis {row} is not unit: {norm}")
        for a in range(3):
            for b in range(a + 1, 3):
                dot = sum(axes[a][k] * axes[b][k] for k in range(3))
                if abs(dot) > 1e-4:
                    fail(f"mesh {mesh_index} principal axes {a}/{b} not orthogonal: {dot}")
        for radius in props.gyration_radii:
            if radius <= 0:
                fail(f"mesh {mesh_index} has non-positive gyration radius {radius}")

    expected = HostMeshProperties()
    status = lib.greyhound_host_mesh_properties(doc, 0, ctypes.byref(expected))
    if status != 0:
        fail(f"greyhound_host_mesh_properties(0): {lib.greyhound_host_last_error().decode()}")
    # Measured on cart-asy (BRepGProp): volume 53605.588 mm3, local centre
    # of gravity OCCT (16.9977, 62.9348, -0.2281) -> Unity (x, z, y) * 0.001.
    if abs(expected.volume_mm3 - 53605.588) > 0.5:
        fail(f"unexpected cart volume {expected.volume_mm3}")
    expected_com = (0.0169977, -0.0002281, 0.0629348)
    actual_com = tuple(expected.centre_of_gravity)
    if any(abs(a - e) > 1e-4 for a, e in zip(actual_com, expected_com)):
        fail(f"unexpected cart centre of gravity {actual_com}, expected {expected_com}")
    print(
        f"properties: cart volume {expected.volume_mm3:.3f} mm3, "
        f"com ({actual_com[0]:.5f} {actual_com[1]:.5f} {actual_com[2]:.5f}) m, "
        f"gyration ({expected.gyration_radii[0]:.2f} {expected.gyration_radii[1]:.2f} "
        f"{expected.gyration_radii[2]:.2f}) mm, file density {expected.file_density}"
    )

    lib.greyhound_host_close_step(doc)
    lib.greyhound_host_free(host)


if __name__ == "__main__":
    main()
