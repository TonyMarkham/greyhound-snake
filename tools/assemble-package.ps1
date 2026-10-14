# Assemble the Windows production packages into dist: for each package,
# stage the authored skeleton plus the fetch config (the package's C#
# provisioner reads its pins from this copy, so the package is
# self-contained), force the native shim rebuild so the fresh shim lands
# in the package, then add the host cdylib (step package only — the
# vision package has no host cdylib yet; crates/opencv-host is planned
# V2). The third-party payloads (Runtime/Plugins/occt,
# Runtime/Plugins/opencv) are NONE of this script's business — not
# asserted, not read, not written; the packages' C# provisioners own
# them exclusively. Windows counterpart of the [linux] assemble-package
# recipe body; it lives in a script because just runs Windows recipe
# lines one process per line, so cross-line variables and multi-line
# blocks never survive in a recipe.
$ErrorActionPreference = "Stop"

$package = "dist/package/com.greyhound.step"
$host_dir = "$package/Runtime/Plugins/x86_64"

# Stage the authored skeleton sources wholesale; the payload and the shim
# (cargo build output) survive untouched.
Copy-Item package/com.greyhound.step/* $package -Recurse -Force

# Stage the fetch config the package's C# provisioner reads: one
# source of pins, refreshed on every assembly.
Copy-Item crates/extern-fetch/config.toml $package/Editor/occt-fetch.toml -Force

# Force build.rs to rerun so the shim rebuilds freshly, mirroring the
# Linux recipe's touch.
(Get-Item crates/occt-sys/build.rs).LastWriteTime = Get-Date
cargo build --release -p importer-host
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$shim = "$package/Runtime/Plugins/shim/x86_64"
# The package payload (Runtime/Plugins/occt) is none of this script's
# business — not asserted, not read, not written; the package's C#
# provisioner owns it exclusively. Only the shim is asserted, because
# this script just
# built it.
if (-not (Test-Path "$shim/greyhound_occt.dll")) {
    throw "missing prerequisite: $shim/greyhound_occt.dll (cargo build produced no shim)"
}

# Create the host directory before copying: Copy-Item with a destination
# that does not exist turns the destination's final path segment into a
# file name, so the dll would land as Plugins/x86_64 (a file) instead of
# Plugins/x86_64/importer_host.dll.
New-Item -ItemType Directory -Force $host_dir | Out-Null
Copy-Item target/release/importer_host.dll $host_dir -Force

# The vision package: the same staging model as the step package —
# authored skeleton wholesale (the built ghcv shim survives), the fetch
# config under the opencv-fetch.toml name its provisioner reads, then a
# forced opencv-sys rebuild so the fresh shim lands in the package. The
# OpenCV payload (Runtime/Plugins/opencv) is none of this script's
# business — the vision package's C# provisioner owns it exclusively.
# No host cdylib copy: the vision package has none yet.
$vision = "dist/package/com.greyhound.vision"
$vision_shim = "$vision/Runtime/Plugins/shim/x86_64"

New-Item -ItemType Directory -Force $vision_shim | Out-Null
Copy-Item package/com.greyhound.vision/* $vision -Recurse -Force
Copy-Item crates/extern-fetch/config.toml $vision/Editor/opencv-fetch.toml -Force

(Get-Item crates/opencv-sys/build.rs).LastWriteTime = Get-Date
cargo build --release -p opencv-sys
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (-not (Test-Path "$vision_shim/ghcv.dll")) {
    throw "missing prerequisite: $vision_shim/ghcv.dll (cargo build produced no shim)"
}

"dist/package assembled:"
Get-ChildItem -Recurse -File dist/package | ForEach-Object { $_.FullName } | Sort-Object
