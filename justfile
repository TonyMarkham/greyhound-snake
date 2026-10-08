# Assemble the production package into dist: skeleton sources plus the
# native payload (host cdylib; shim and OCCT install compile in place).
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

    # Clear only the files this recipe owns. The OCCT install tree is a
    # cmake artifact and must survive assembly.
    rm -f "$package/package.json" "$package/Third Party Notices.md"
    rm -f "$host/libimporter_host.so"
    mkdir -p "$shim" "$host"

    cp package/com.greyhound.step/package.json "$package/package.json"
    cp "package/com.greyhound.step/Third Party Notices.md" "$package/Third Party Notices.md"

    touch crates/occt-sys/build.rs
    cargo build --release -p importer-host

    cp -L "target/release/libimporter_host.so" "$host/libimporter_host.so"

    echo "dist/package assembled:"
    find dist/package -type f | sort

# Verify the assembled package from a copied layout: replays the C# ABI flow
# (dlopen host -> version -> new -> open STEP -> counts/fill) with ctypes.
verify-package: assemble-package
    python3 tools/verify-package.py