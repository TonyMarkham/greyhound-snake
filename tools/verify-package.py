#!/usr/bin/env python3
"""Verify the assembled native package from a copied layout.

Replays the exact C# consumption flow of the importer-host ABI against a
temporary copy of package/ so path independence is actually exercised:
dlopen host -> version probe -> greyhound_host_new(occt_dir, shim) ->
open_step -> mesh_counts -> mesh_fill.

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

EXPECTED_VERTEX_COUNT = 1744
EXPECTED_INDEX_COUNT = 5580
EXPECTED_SUBMESH_COUNT = 1
EXPECTED_COLOR_COUNT = 1
EXPECTED_COLOR_SRGB = (0.976470577825, 0.678431390124, 0.121568629232)

DEFLECTION = 0.01
ANGLE_RAD = 0.5
SCALE = 0.001

HOST_ABI_VERSION = 2


class UnityBounds(ctypes.Structure):
    _fields_ = [("min", ctypes.c_float * 3), ("max", ctypes.c_float * 3)]


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
    parser.add_argument("--asset", type=Path, default=Path("assets/rod-clamp-16mm.stp"))
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
    lib.greyhound_host_mesh_counts.restype = ctypes.c_int32
    lib.greyhound_host_mesh_counts.argtypes = [
        ctypes.c_void_p,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.POINTER(HostMeshCounts),
    ]
    lib.greyhound_host_mesh_fill.restype = ctypes.c_int32
    lib.greyhound_host_mesh_fill.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(UnityVertex),
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(UnitySubMesh),
        ctypes.POINTER(ctypes.c_uint32),
        ctypes.POINTER(ctypes.c_float),
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

    counts = HostMeshCounts()
    status = lib.greyhound_host_mesh_counts(
        doc, DEFLECTION, ANGLE_RAD, SCALE, ctypes.byref(counts)
    )
    if status != 0:
        fail(f"greyhound_host_mesh_counts: {lib.greyhound_host_last_error().decode()}")
    if (
        counts.vertex_count != EXPECTED_VERTEX_COUNT
        or counts.index_count != EXPECTED_INDEX_COUNT
        or counts.submesh_count != EXPECTED_SUBMESH_COUNT
        or counts.color_count != EXPECTED_COLOR_COUNT
    ):
        fail(
            f"unexpected counts: {counts.vertex_count} verts, "
            f"{counts.index_count} indices, {counts.submesh_count} submeshes, "
            f"{counts.color_count} colors"
        )
    print(
        f"counts: {counts.vertex_count} verts, {counts.index_count} indices, "
        f"{counts.submesh_count} submeshes, bounds "
        f"[{counts.bounds.min[0]:.4f} {counts.bounds.min[1]:.4f} "
        f"{counts.bounds.min[2]:.4f}] .. "
        f"[{counts.bounds.max[0]:.4f} {counts.bounds.max[1]:.4f} "
        f"{counts.bounds.max[2]:.4f}]"
    )

    verts = (UnityVertex * counts.vertex_count)()
    indices = (ctypes.c_uint32 * counts.index_count)()
    submeshes = (UnitySubMesh * counts.submesh_count)()
    submesh_colors = (ctypes.c_uint32 * counts.submesh_count)()
    colors = (ctypes.c_float * (counts.color_count * 4))()
    status = lib.greyhound_host_mesh_fill(doc, verts, indices, submeshes, submesh_colors, colors)
    if status != 0:
        fail(f"greyhound_host_mesh_fill: {lib.greyhound_host_last_error().decode()}")

    covered = sum(submesh.index_count for submesh in submeshes)
    if covered != counts.index_count:
        fail(f"submesh index ranges cover {covered} of {counts.index_count} indices")

    if counts.submesh_count > 0 and counts.color_count > 0:
        color_index = submesh_colors[0]
        actual = tuple(colors[color_index * 4 + component] for component in range(3))
        if any(abs(a - e) > 1e-3 for a, e in zip(actual, EXPECTED_COLOR_SRGB)):
            fail(f"unexpected submesh color: {actual}, expected {EXPECTED_COLOR_SRGB}")
        print(
            f"colors: submesh 0 -> color {color_index} sRGB "
            f"({actual[0]:.3f} {actual[1]:.3f} {actual[2]:.3f})"
        )

    first = verts[0]
    magnitude = sum(component * component for component in first.normal) ** 0.5
    if abs(magnitude - 1.0) > 1e-3:
        fail(f"first vertex normal is not unit length: {magnitude}")
    print(
        f"fill: first vertex position "
        f"[{first.position[0]:.6f} {first.position[1]:.6f} "
        f"{first.position[2]:.6f}], normal magnitude {magnitude:.6f}"
    )

    lib.greyhound_host_close_step(doc)
    lib.greyhound_host_free(host)


if __name__ == "__main__":
    main()