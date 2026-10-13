# Assemble the Windows production package into dist: stage the authored
# skeleton, force an occt-sys rebuild so build.rs restages the runtime
# payload (the shim's DLL closure + licenses) and the shim itself from the
# neutral install (occt-win.md step 2), then add the host cdylib. Windows
# counterpart of the [linux] assemble-package recipe body; it lives in a
# script because just runs Windows recipe lines one process per line, so
# cross-line variables and multi-line blocks never survive in a recipe.
$ErrorActionPreference = "Stop"

$package = "dist/package/com.greyhound.step"
$host_dir = "$package/Runtime/Plugins/x86_64"

# Stage the authored skeleton sources wholesale; the payload (build.rs
# output) and the shim (cargo build output) survive untouched.
Copy-Item package/com.greyhound.step/* $package -Recurse -Force

# Force build.rs to rerun so the payload restages freshly, mirroring the
# Linux recipe's touch.
(Get-Item crates/occt-sys/build.rs).LastWriteTime = Get-Date
cargo build --release -p importer-host
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$payload_lib = "$package/Runtime/Plugins/occt/x86_64/lib"
$shim = "$package/Runtime/Plugins/shim/x86_64"
foreach ($required in @("$payload_lib/TKernel.dll", "$shim/greyhound_occt.dll")) {
    if (-not (Test-Path $required)) {
        throw "missing prerequisite: $required; install OCCT per occt-win.md step 2, the neutral install build.rs stages from"
    }
}

# Create the host directory before copying: Copy-Item with a destination
# that does not exist turns the destination's final path segment into a
# file name, so the dll would land as Plugins/x86_64 (a file) instead of
# Plugins/x86_64/importer_host.dll.
New-Item -ItemType Directory -Force $host_dir | Out-Null
Copy-Item target/release/importer_host.dll $host_dir -Force

"dist/package assembled:"
Get-ChildItem -Recurse -File dist/package | ForEach-Object { $_.FullName } | Sort-Object
