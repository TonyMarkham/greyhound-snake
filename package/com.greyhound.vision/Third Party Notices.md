# Third-party notices

This software uses **OpenCV** for a part of its functionality: the
ArUco marker detection behind the ghcv shim (`crates/opencv-sys`).

| Component | Version | License |
|---|---|---|
| OpenCV | 5.0.0 | Apache License 2.0 |

- The shipped binaries come from the pinned `opencv-dev` release
  archives (repo `TonyMarkham/opencv-dev`, tag `v0.1.0`). The license
  texts ship with the binaries inside the package payload
  (`Runtime/Plugins/opencv/<arch>/lib/`, provisioned by the package's
  C# provisioner).
- The C++ shim in `crates/opencv-sys/cpp/` includes OpenCV header
  files; the object code generated from them is distributed under this
  repository's MIT license (`LICENSE`) with this notice retained, as
  Apache License 2.0 permits.