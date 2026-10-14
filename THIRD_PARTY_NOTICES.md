# Third-party notices

This software uses **Open CASCADE Technology (OCCT)** for a part of its
functionality: STEP file reading and the geometry and tessellation services
behind it.

| Component | Version | License |
|---|---|---|
| Open CASCADE Technology | 8.0.1 (tag `V8.0.1`) | LGPL-2.1 with the Open CASCADE exception |

- The license texts are bundled in `third_party/occt/`
  (`LICENSE_LGPL_21.txt`, `OCCT_LGPL_EXCEPTION.txt`) and ship with the
  binaries inside the package payload
  (`dist/package/com.greyhound.step/Runtime/Plugins/occt/x86_64/lib/`).
- The shipped binaries come from the pinned `occt-dev` release archives
  (repo `TonyMarkham/occt-dev`, tag `v0.1.0`), built from the `V8.0.1`
  release tag with one local source modification:
  `adm/cmake/occt_toolkit.cmake` no longer sets the `VERSION`/`SOVERSION`
  target properties — the toolkits build as plain unversioned `libTK*.so`
  files. The OCCT libraries are linked dynamically and remain replaceable.
- The package ships the OCCT binaries. LGPL-2.1 requires accompanying them
  with the complete corresponding OCCT source, including the modification
  recorded above (or a written offer for it), together with these license
  texts and this notice.

The C++ shims in `crates/occt-sys/cpp/` include OCCT header files; per the
Open CASCADE exception, the object code generated from them is distributed
under this repository's MIT license (`LICENSE`).

This software embeds **OpenCV's predefined 4x4 ArUco dictionary data** for a
part of its functionality: the marker bit patterns emitted by the
`crates/aruco` SVG generator.

| Component | Version | License |
|---|---|---|
| OpenCV `DICT_4X4_1000_BYTES` table | branch `4.x`, commit `802cf95aa3d42816995695bf68a86b3532f12760` | Apache-2.0 |

- Source:
  `modules/objdetect/src/aruco/predefined_dictionaries.hpp` in
  [opencv/opencv](https://github.com/opencv/opencv); transcribed verbatim by
  `tools/extract_aruco_dict.py` into `crates/aruco/src/data/dict_4x4_1000.rs`
  (byte values unchanged; the C array became a Rust `const`).
- OpenCV is licensed under the Apache License, Version 2.0. A copy of the
  license is bundled in `third_party/opencv/LICENSE` and the transcribed
  module's doc-comment carries the attribution.
