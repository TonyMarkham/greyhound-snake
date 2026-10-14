# Recipes carry [linux]/[windows] attributes with no fallback: an
# unexercised platform fails fast instead of silently running a body
# written for another OS (AGENTS.md: the macOS integration is not yet
# exercised). just runs Windows recipe lines one process per line, so
# Windows bodies stay single-line launchers (multi-statement logic lives
# in tools/*.ps1); Linux bodies are shebang scripts.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Assemble the production package into dist: skeleton sources, the fetch
# config the package's C# provisioner reads, and the native payload (host
# cdylib; shim and OCCT install compile in place). The payload's lib/ is
# NOT a build output — on Windows it is provisioned by `just fetch-occt`
# (extern-fetch, [occt.windows.payload]); on Linux the OCCT install IS
# the payload (occt-linux.md).
[linux]
assemble-package:
    #!/usr/bin/env bash
    set -euo pipefail

    package="dist/package/com.greyhound.step"
    occt="$package/Runtime/Plugins/occt/x86_64"
    shim="$package/Runtime/Plugins/shim/x86_64"
    host="$package/Runtime/Plugins/x86_64"

    for required in "$occt/lib/libTKernel.so" "$occt/include/opencascade/Standard_Version.hxx"; do
        if [ ! -e "$required" ]; then
            echo "missing prerequisite: $required" >&2
            echo "install OCCT per occt-linux.md (INSTALL_DIR is the package occt directory)" >&2
            exit 1
        fi
    done

    # Stage the authored skeleton sources wholesale; the OCCT install tree
    # (cmake) and the shim (cargo build output) survive untouched.
    rm -f "$host/libimporter_host.so"
    mkdir -p "$shim" "$host"

    cp -r package/com.greyhound.step/. "$package/"
    cp crates/extern-fetch/config.toml "$package/Editor/occt-fetch.toml"

    touch crates/occt-sys/build.rs
    cargo build --release -p importer-host

    cp -L "target/release/libimporter_host.so" "$host/libimporter_host.so"

    echo "dist/package assembled:"
    find dist/package -type f | sort

# Windows counterpart of assemble-package: stages the authored skeleton,
# the fetch config, and the host cdylib; build.rs rebuilds the shim into
# the package. The OCCT payload lib/ is not built here — the prerequisite
# asserts it was provisioned by `just fetch-occt` (extern-fetch,
# [occt.windows.payload]). The body is a script launcher because just
# runs Windows recipe lines one process per line: cross-line variables
# and multi-line blocks do not survive in a recipe body; the logic lives
# in tools/assemble-package.ps1.
[windows]
assemble-package:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/assemble-package.ps1

# Verify the assembled package from a copied layout: replays the C# ABI flow
# (dlopen host -> version -> new -> open STEP -> counts/fill) with ctypes.
[linux]
verify-package: assemble-package
    python3 tools/verify-package.py

# Windows variant of verify-package: CPython is `python` here, not `python3`.
[windows]
verify-package: assemble-package
    python tools/verify-package.py

# Fetch the pinned occt-dev release archives into the extern/ cache
# (refactor-repo.md D3/D4): download -> sha256 verify -> extract ->
# provenance marker. Unpinned entries extract + report the hash and fail
# the run; pin the reported hash into crates/extern-fetch/config.toml.
[windows]
[linux]
fetch-occt:
    cargo run -p extern-fetch -- --group occt
