# Assemble the Windows production package into dist: stage the authored
# skeleton plus the fetch config (the package's C# provisioner reads its
# pins from this copy, so the package is self-contained), force an
# occt-sys rebuild so the freshly built shim lands in the package, then
# add the host cdylib. The OCCT payload (Runtime/Plugins/occt) is NONE
# of this script's business — not asserted, not read, not written; the
# package's C# provisioner owns it exclusively (refactor-repo.md D8,
# amended 2026-10-14). Windows counterpart of the [linux]
# assemble-package recipe body; it lives in a script because just runs
# Windows recipe lines one process per line, so cross-line variables and
# multi-line blocks never survive in a recipe.
$ErrorActionPreference = "Stop"

$package = "dist/package/com.greyhound.step"
$host_dir = "$package/Runtime/Plugins/x86_64"

# Stage the authored skeleton sources wholesale; the payload and the shim
# (cargo build output) survive untouched.
Copy-Item package/com.greyhound.step/* $package -Recurse -Force

# Stage the fetch config the package's C# provisioner reads
# (refactor-repo.md D8): one source of pins, refreshed on every assembly.
Copy-Item crates/extern-fetch/config.toml $package/Editor/occt-fetch.toml -Force

# Force build.rs to rerun so the shim rebuilds freshly, mirroring the
# Linux recipe's touch.
(Get-Item crates/occt-sys/build.rs).LastWriteTime = Get-Date
cargo build --release -p importer-host
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$shim = "$package/Runtime/Plugins/shim/x86_64"
# The package payload (Runtime/Plugins/occt) is none of this script's
# business — not asserted, not read, not written; the package's C#
# provisioner owns it exclusively (refactor-repo.md D8, amended
# 2026-10-14). Only the shim is asserted, because this script just
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

"dist/package assembled:"
Get-ChildItem -Recurse -File dist/package | ForEach-Object { $_.FullName } | Sort-Object
