# Recipes carry per-OS platform attributes ([linux]/[windows]/[macos])
# with no fallback: an unexercised platform fails fast instead of
# silently running a body written for another OS. just runs Windows
# recipe lines one process per line (in PowerShell, where `cp` is
# Copy-Item), so every body here is single-line commands that read the
# same in bash and PowerShell.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Copy the fetch config into the two tracked packages, where their C#
# provisioners read it (Editor/occt-fetch.toml, Editor/opencv-fetch.toml).
# The provisioners never read crates/extern-fetch/config.toml itself, so
# run this after every pin change, or the packages keep the old pins
# (a missing greyhound_* entry silently skips that library's download).
[windows]
[linux]
[macos]
sync-fetch-config:
    cp crates/extern-fetch/config.toml package/com.greyhound.step/Editor/occt-fetch.toml
    cp crates/extern-fetch/config.toml package/com.greyhound.vision/Editor/opencv-fetch.toml

# Verify the tracked step package from a copied layout: replays the C#
# ABI flow (dlopen host -> version -> new -> open STEP -> counts/fill)
# with ctypes. Needs the host and shim the package's provisioner
# downloads into package/com.greyhound.step/Runtime/Plugins: open a Unity
# project that uses the package once first. macOS mirrors the Linux
# body: python3 plus the script's own per-OS handling of the host
# filename and the runtime directory.
[linux]
[macos]
verify-package:
    python3 tools/verify-package.py

# Windows variant of verify-package: CPython is `python` here, not `python3`.
[windows]
verify-package:
    python tools/verify-package.py

# Fetch every pinned group's dev trees into the extern/ cache
# (occt + opencv): download -> sha256 verify -> extract -> provenance
# marker, per group. Dev trees only: package payloads are provisioned
# exclusively by the packages' C# provisioners, never by this recipe
# (the config's payload entries are inert here). Unpinned entries
# extract + report the hash and fail the run; pin the reported hash
# into crates/extern-fetch/config.toml.
[windows]
[linux]
[macos]
fetch:
    cargo run -p extern-fetch -- --group occt
    cargo run -p extern-fetch -- --group opencv

# Fetch the pinned opencv-dev release archives into the extern/ cache:
# download -> sha256 verify -> extract -> provenance marker. Dev trees
# only: the package payload is provisioned exclusively by the vision
# package's C# provisioner, never by this recipe (the config's payload
# entries are inert here). Unpinned entries extract + report the hash
# and fail the run; pin the reported hash into
# crates/extern-fetch/config.toml.
[windows]
[linux]
[macos]
fetch-opencv:
    cargo run -p extern-fetch -- --group opencv

# Fetch the pinned occt-dev release archives into the extern/ cache:
# download -> sha256 verify -> extract ->
# provenance marker. Dev trees only: the package payload is provisioned
# exclusively by the package's C# provisioner, never by this recipe.
# Unpinned entries extract + report the hash and fail the run; pin the
# reported hash into crates/extern-fetch/config.toml.
[windows]
[linux]
[macos]
fetch-occt:
    cargo run -p extern-fetch -- --group occt

# The full local gate, in dependency order: the native shims first
# (each build.rs compiles its C++ against the extern/ dev trees — run
# `just fetch` once before the first build on a new checkout), then the
# workspace-wide checks in the order each catches its class of failure
# cheapest: fmt (syntax/style), check (types), clippy (lint, including
# the deny-panic workspace lints), test (behavior), then the plain
# debug and release builds (feature unification + release-lint paths
# the earlier steps don't compile). Exits on the first failure.
[windows]
[linux]
[macos]
validate:
    cargo build -p occt-sys
    cargo build -p opencv-sys
    cargo build -p importer-host
    cargo fmt
    cargo check
    cargo clippy --workspace --all-targets
    cargo test
    cargo build
    cargo build --release
