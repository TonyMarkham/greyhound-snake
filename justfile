# Recipes carry per-OS platform attributes ([linux]/[windows]/[macos])
# with no fallback: an unexercised platform fails fast instead of
# silently running a body written for another OS. just runs Windows
# recipe lines one process per line, so Windows bodies stay single-line
# launchers (multi-statement logic lives in tools/*.ps1); Linux and
# macOS bodies are shebang scripts — one shared body where only the
# cdylib filename differs, branching on `uname`.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Assemble the production package into dist: stage the authored skeleton
# sources plus the fetch config the package's C# provisioner reads, then
# add the host cdylib; build.rs rebuilds the shim into the package. The
# OCCT payload (Runtime/Plugins/occt) is NONE of this recipe's business
# on any platform — not asserted, not read, not written; the package's
# C# provisioner owns it exclusively (refactor-repo.md D8, amended
# 2026-10-14).
[linux]
[macos]
assemble-package:
    #!/usr/bin/env bash
    set -euo pipefail

    package="dist/package/com.greyhound.step"

    # The cdylib filename and the plugin-architecture directory name
    # are the only per-OS differences in the staging (Darwin: .dylib
    # and arm64 — Unity's Apple Silicon plugin folder; an arm64 editor
    # never loads from the Intel-only x86_64 folder). Host, shim, and
    # payload directories all carry the same <arch> name; only the
    # host's is resolved by Unity as a plugin, the other two are
    # dlopen'd by explicit path.
    case "$(uname)" in
        Darwin) extension="dylib"; archdir="arm64" ;;
        *)      extension="so"; archdir="x86_64" ;;
    esac
    host="$package/Runtime/Plugins/$archdir"
    shim="$package/Runtime/Plugins/shim/$archdir"

    # Stage the authored skeleton sources wholesale; the provisioned
    # payload and the shim (cargo build output) survive untouched. The
    # steps below the shim check are non-destructive, and the host copy
    # overwrites in place — a failed run must leave the previously
    # assembled package intact (the Windows body's rule).
    mkdir -p "$shim" "$host"

    cp -r package/com.greyhound.step/. "$package/"
    cp crates/extern-fetch/config.toml "$package/Editor/occt-fetch.toml"

    touch crates/occt-sys/build.rs
    cargo build --release -p importer-host

    # The package payload (Runtime/Plugins/occt) is none of this
    # recipe's business — not asserted, not read, not written; the
    # package's C# provisioner owns it exclusively (refactor-repo.md
    # D8, amended 2026-10-14). Only the shim is asserted, because this
    # recipe just built it.
    if [ ! -e "$shim/libgreyhound_occt.$extension" ]; then
        echo "missing prerequisite: $shim/libgreyhound_occt.$extension (cargo build produced no shim)" >&2
        exit 1
    fi

    cp "target/release/libimporter_host.$extension" "$host/libimporter_host.$extension"

    # The listing prunes Plugins/occt entirely (-prune: no descent, no
    # read inside it) — the recipe's own products are what it names.
    echo "dist/package assembled:"
    find dist/package -path '*Runtime/Plugins/occt*' -prune -o -type f -print | sort

# Windows counterpart of assemble-package: stages the authored skeleton,
# the fetch config, and the host cdylib; build.rs rebuilds the shim into
# the package. The body is a script launcher because just runs Windows
# recipe lines one process per line: cross-line variables and multi-line
# blocks do not survive in a recipe body; the logic lives in
# tools/assemble-package.ps1.
[windows]
assemble-package:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/assemble-package.ps1

# Verify the assembled package from a copied layout: replays the C# ABI flow
# (dlopen host -> version -> new -> open STEP -> counts/fill) with ctypes.
# macOS mirrors the Linux body: python3 plus the script's own per-OS
# handling of the host filename and the runtime directory.
[linux]
[macos]
verify-package: assemble-package
    python3 tools/verify-package.py

# Windows variant of verify-package: CPython is `python` here, not `python3`.
[windows]
verify-package: assemble-package
    python tools/verify-package.py

# Fetch the pinned occt-dev release archives into the extern/ cache
# (refactor-repo.md D3/D4): download -> sha256 verify -> extract ->
# provenance marker. Dev trees only: the package payload is provisioned
# exclusively by the package's C# provisioner, never by this recipe.
# Unpinned entries extract + report the hash and fail the run; pin the
# reported hash into crates/extern-fetch/config.toml.
[windows]
[linux]
[macos]
fetch-occt:
    cargo run -p extern-fetch -- --group occt
