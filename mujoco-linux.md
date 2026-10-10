# Installing MuJoCo on Linux for the simulation side

This repository exports **MuJoCo MJCF models** from Unity (the joint set's
Export button). The simulation side — validating exported XML, the future
training loop's MuJoCo workers — needs a local MuJoCo installation on the
**x86_64 Linux** machine. MuJoCo is **not** a Unity-package payload: unlike
the OCCT binaries (`occt-linux.md`), it ships in no UPM package and is never
redistributed to Unity users.

**Policy: install from the official release artifact. Build from source
only when no artifact is available.** Every MuJoCo GitHub release ships
prebuilt tarballs for Linux x86_64 with a published SHA-256 sidecar, so the
whole install is download → verify → extract — no build toolchain, no pip
(this repository does not install anything with pip), no network fetch at
build time.

- Repo: <https://github.com/google-deepmind/mujoco>
- Release: **3.15.0** — <https://github.com/google-deepmind/mujoco/releases/tag/3.15.0>
  (the pin recorded in `mujoco-schema.md`; readthedocs "stable" tracks it)
- License: Apache-2.0 (`LICENSE` in the tarball; keep it alongside the
  extracted files)

> ## Prerequisites
>
> - An x86_64 Linux machine with the usual desktop runtime libraries —
>   `simulate` is an OpenGL/X11 GUI application. A normal desktop has these;
>   on a minimal server or container install them first:
>
>   ```
>   sudo apt install libgl1 libx11-6 libxcursor1 libxrandr2 libxinerama1 libxi6
>   ```
>
>   (Fedora: `mesa-libGL libX11 libXcursor libXrandr libXinerama libXi`.)
>   No compiler, no CMake, no Python — the artifact is self-contained.
> - `wget`/`curl`, `tar`, and `sha256sum` (in coreutils).

> ## 1. Download the release artifact and its checksum
>
> Run all shell commands from the repository root; nothing in this guide
> changes the working directory. Download into `~/Downloads`:
>
> ```
> wget -P ~/Downloads \
>     https://github.com/google-deepmind/mujoco/releases/download/3.15.0/mujoco-3.15.0-linux-x86_64.tar.gz \
>     https://github.com/google-deepmind/mujoco/releases/download/3.15.0/mujoco-3.15.0-linux-x86_64.tar.gz.sha256
> ```

> ## 2. Verify the checksum
>
> The `.sha256` sidecar is a standard `sha256sum` line (hash + file
> name); it is the release page's independent statement of the artifact's
> digest. The published hash for the 3.15.0 artifact is
> `319943b332fd84c968f655f695d8f4b03cc6a68f658bacd0591f0a552744e497`.
> Verify the download against it without changing directories:
>
> ```
> sha256sum ~/Downloads/mujoco-3.15.0-linux-x86_64.tar.gz |
>     grep -q 319943b332fd84c968f655f695d8f4b03cc6a68f658bacd0591f0a552744e497 \
>     && echo VERIFIED
> ```
>
> It prints `VERIFIED` on success and fails silently otherwise. If it
> does not print, do not use the download; re-download or investigate
> the mirror.

> ## 3. Extract into the repo's dist
>
> Run from the repository root. Extract into
> `dist/mujoco/x86_64-linux/<version>` — a repo-local, gitignored prefix
> in the spirit of the OCCT dist (but separate: MuJoCo is a
> development/simulation dependency, never a Unity-package payload). The
> tarball's single top-level directory is `mujoco-3.15.0`, so
> `--strip-components=1` lands its contents directly in the
> version-named directory:
>
> ```
> mkdir -p dist/mujoco/x86_64-linux/3.15.0
> tar -xzf ~/Downloads/mujoco-3.15.0-linux-x86_64.tar.gz \
>     -C dist/mujoco/x86_64-linux/3.15.0 --strip-components=1
> ```
>
> Installed layout (read back from the 3.15.0 artifact):
>
> ```
> dist/mujoco/x86_64-linux/3.15.0/
> ├── bin/                  simulate, compile, bin/assets/ (runtime materials), mujoco_plugin/*.so
> ├── lib/                  libmujoco.so (symlink) → libmujoco.so.3.15.0 (real file)
> ├── include/mujoco/       the C API headers (88 files)
> ├── model/                the bundled model collection (humanoid, …)
> ├── sample/, simulate/    sample and simulate sources with their CMake setups
> ├── wasm/                 WebAssembly support files
> ├── LICENSE               Apache-2.0
> └── THIRD_PARTY_NOTICES
> ```
>
> `simulate` runs in place: its ELF `RUNPATH` is `$ORIGIN/../lib`, so the
> `bin/simulate` executable resolves `libmujoco.so.3.15.0` from the
> sibling `lib/` — nothing needs to be installed system-wide and the tree
> is relocatable, provided it is kept intact. (`libmujoco.so` is a
> symlink inside the artifact — fine for this development install; the
> real-files-only rule applies to the Unity package payload, not here.
> If some tool insists on a real file, copy it.)

> ## 4. Validate
>
> Run the bundled viewer on a bundled model first — this proves the
> artifact's loader, rendering, and GUI work on your machine:
>
> ```
> dist/mujoco/x86_64-linux/3.15.0/bin/simulate \
>     dist/mujoco/x86_64-linux/3.15.0/model/humanoid/humanoid.xml
> ```
>
> A window should open with the humanoid standing under gravity; closing it
> writes `MUJOCO_LOG.TXT` into the current directory (harmless, deletable).
>
> Then the real target of this install — the Unity exporter's output, the
> M3 smoke test that was waiting on MuJoCo. The export lives beside the
> STEP file in the Unity project (`~` folder, Unity-ignored):
>
> ```
> dist/mujoco/x86_64-linux/3.15.0/bin/simulate \
>     /home/tony/unity/My\ project/Assets/cart-pole-asm.stp~/cart-pole-asm.xml
> ```
>
> The assembly must render and look like the STEP in a CAD viewer (the
> same visual-fidelity debugging rule as everywhere else in this repo: any
> mirroring is a bug in the exporter, never in the data). In `simulate`,
> open the Control panel and drive `cart-asy.actuator` — the cart should
> slide along the rail and the pole should swing on its hinge while the
> welded parts stay rigid.
>
> **What this validation does not cover:** the numeric model checks
> (body/mesh/actuator counts, Σ `body_mass`) come from the Python
> bindings, which this guide deliberately does not install — pip is out
> of policy, and whether to build the bindings locally from the pinned
> source is a separate, open decision. Until that decision, `simulate` is
> the validation surface.

> ## Swapping versions
>
> Replace `3.15.0` in the step-1 URLs and the step-3 directory name; the
> `.sha256` sidecar pins the artifact's integrity. Old version directories
> can be deleted; nothing else references them except the validating
> commands. Readthedocs "stable" tracks the latest release — when it moves
> past the pin, re-check `mujoco-schema.md` before bumping.

> ## Troubleshooting
>
> - **`simulate` fails to start with GL/X11 errors** — the runtime
>   libraries from Prerequisites are missing (typically on servers/WSL);
>   install the desktop GL/X11 runtime packages listed there.
> - **`simulate: error while loading shared libraries: libmujoco.so.3.15.0`**
>   — the extracted tree is incomplete or was moved piecemeal;
>   `bin/simulate` resolves the library from the sibling `lib/` through
>   its `$ORIGIN/../lib` RUNPATH, so keep the extracted tree intact and
>   invoke `bin/simulate` in place.
> - **Checksum mismatch** — re-download; compare the digest against the
>   value recorded in step 2. Never install an artifact that fails
>   verification.
> - **`MUJOCO_LOG.TXT` appears in unexpected directories** — MuJoCo writes
>   it into the process's working directory on warnings/errors; it is a
>   log, not an installation problem.

> ## Appendix: building from source (fallback only)
>
> Run from the repository root. Use this only when no release artifact fits the machine. Per
> `doc/programming` at the tag: CMake ≥ 3.16 and a C++17 compiler; the
> build auto-fetches its dependencies from upstream repositories over the
> network via CMake `FetchContent` (qhull, tinyxml2, libccd, miniz, GLFW
> for `simulate`; GTest and google-benchmark only if tests are enabled),
> so git and network access are required at configure time. Stock steps:
>
> ```
> git clone --depth 1 --branch 3.15.0 https://github.com/google-deepmind/mujoco.git tmp/mujoco/mujoco
> cmake -S tmp/mujoco/mujoco -B tmp/mujoco/build -G Ninja \
>       -DCMAKE_BUILD_TYPE=Release \
>       -DCMAKE_INSTALL_PREFIX="$PWD/dist/mujoco/x86_64-linux/3.15.0" \
>       -DMUJOCO_BUILD_TESTS=OFF
> cmake --build tmp/mujoco/build --parallel
> cmake --install tmp/mujoco/build
> ```
>
> Stock options, none modified: `-DMUJOCO_BUILD_TESTS=OFF` skips the test
> dependencies (the library is still fully built and installed); leave
> `-DMUJOCO_BUILD_SIMULATE=ON` (default) so the viewer installs into
> `bin/`. `-DMUJOCO_ENABLE_AVX` defaults to ON — the binaries then require
> AVX at runtime; set `MUJOCO_ENABLE_AVX=OFF` only if the target machine
> lacks AVX. There is no local source modification, unlike the OCCT build.
> `cmake --install` lays out `bin/`, `include/mujoco/`, `lib/`, and the
> model collection under `share/mujoco/` — the validating commands above
> need the `bin/simulate` path only.

> ## References
>
> - Build-from-source and distribution layout: MuJoCo documentation,
>   *Programming → Building from source*
>   (<https://mujoco.readthedocs.io/en/stable/programming.html>) — checked
>   against `doc/programming/index.rst`, `CMakeLists.txt`,
>   `cmake/MujocoOptions.cmake`, `cmake/MujocoDependencies.cmake`, and
>   `simulate/cmake/SimulateDependencies.cmake` at tag **3.15.0**
> - Release assets and checksums:
>   <https://github.com/google-deepmind/mujoco/releases/tag/3.15.0>
>   (`mujoco-3.15.0-linux-x86_64.tar.gz.sha256`, fetched and recorded
>   2026-10-10)
> - Repo docs this install serves: `unity-mujoco.md` (M3 exporter),
>   `mujoco-schema.md` (schema facts), `potpori.md` (sim control model;
>   MuJoCo install was its open verification item 6)
