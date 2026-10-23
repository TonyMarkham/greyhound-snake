# Installing MuJoCo on Windows for the simulation side

This repository exports **MuJoCo MJCF models** from Unity (the joint set's
Export button). The simulation side — validating exported XML, the future
training loop's MuJoCo workers — needs a local MuJoCo installation on the
**x86_64 Windows** machine. MuJoCo is **not** a Unity-package payload:
unlike the OCCT binaries, it ships in no UPM package and is never
redistributed to Unity users. This is the Windows counterpart of
`mujoco-linux.md`; it differs only where the platform does.

**Status (2026-10-22): steps 1–4 and the appendix have not been executed
yet.** The artifact facts below were read back from the release (its
asset metadata, checksum sidecar, and the zip's central directory), not
from an extracted install. The prerequisite checks were run on the
reference machine.

**Policy: install from the official release artifact. Build from source
only when no artifact is available.** The 3.15.0 release ships a prebuilt
zip for Windows x86_64 with a published SHA-256 sidecar, so the whole
install is download → verify → extract — no installer (MuJoCo's docs: "a
single .zip on Windows … There is no installer"), no build toolchain, no
pip (this repository does not install anything with pip). Nothing in this
guide changes the system: the only writes are the download into
`Downloads` and the extraction into the repo's gitignored `dist\`.

- Repo: <https://github.com/google-deepmind/mujoco>
- Release: **3.15.0** — <https://github.com/google-deepmind/mujoco/releases/tag/3.15.0>
  (the pin recorded in `mujoco-schema.md`; readthedocs "stable" tracks it)
- License: Apache-2.0 (`LICENSE` in the zip; keep it alongside the
  extracted files)

> ## Prerequisites
>
> Three things the machine must already provide, each with a read-only
> check for PowerShell (Windows PowerShell 5.1 or PowerShell 7). The
> checks read; they change nothing. **If a check fails, stop** — this guide
> does not install or repair system components.
>
> - **A CPU with AVX.** MuJoCo "requires a processor with AVX
>   instructions" (MuJoCo docs, *Programming*). The check asks Windows;
>   `39` is `PF_AVX_INSTRUCTIONS_AVAILABLE`, answered on Windows 10 2004
>   and later:
>
>   ```
>   Add-Type -Namespace Win32 -Name Cpu -MemberDefinition '[DllImport("kernel32.dll")] public static extern bool IsProcessorFeaturePresent(uint feature);'
>   "AVX: " + [Win32.Cpu]::IsProcessorFeaturePresent(39)
>   ```
>
>   Pass: `AVX: True`.
> - **The Visual C++ runtime DLLs.** The prebuilt binaries need nothing
>   "outside the standard C runtime" (MuJoCo docs, *Getting started*); in
>   the 3.15.0 import tables of `simulate.exe` and `mujoco.dll` that
>   runtime is these four DLLs (every other import is a Windows system
>   DLL):
>
>   ```
>   'MSVCP140.dll', 'MSVCP140_ATOMIC_WAIT.dll', 'VCRUNTIME140.dll', 'VCRUNTIME140_1.dll' |
>       ForEach-Object {
>           $path = "$env:SystemRoot\System32\$_"
>           if (Test-Path $path) { '{0,-26} {1}' -f $_, (Get-Item $path).VersionInfo.ProductVersion }
>           else { '{0,-26} MISSING' -f $_ }
>       }
>   ```
>
>   Pass: four lines with a version, none `MISSING`.
> - **An OpenGL driver.** `simulate` opens an OpenGL window (its README)
>   and MuJoCo's classic renderer uses OpenGL 1.5 (MuJoCo docs,
>   *Programming*). On Windows the GPU's OpenGL implementation is its
>   driver's ICD, registered as `OpenGLDriverName` under the display
>   adapter's class key (Microsoft docs, *Loading an OpenGL Installable
>   Client Driver*):
>
>   ```
>   Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' |
>       Select-Object DriverDesc, @{ n = 'OpenGLDriverName'; e = { $_.OpenGLDriverName -join ',' } } |
>       Format-List
>   ```
>
>   Pass: at least one adapter with a non-empty `OpenGLDriverName`.
>   Adapters without one (virtual displays) are listed too and do not
>   matter.
>
> Tools: `curl.exe` — written with `.exe` because Windows PowerShell 5.1
> aliases `curl` to `Invoke-WebRequest`; `(Get-Command curl.exe).Source`
> prints its path — plus the built-in cmdlets `Get-FileHash` and
> `Expand-Archive`. No compiler, no CMake, no Python.
>
> Reference machine (2026-10-22, Windows 11, Windows PowerShell 5.1):
> `AVX: True`; all four DLLs `14.51.36247.0`; `NVIDIA GeForce RTX 3090`
> with `...\nvoglv64.dll`.

> ## 1. Download the release artifact and its checksum
>
> Run all shell commands in PowerShell from the repository root; nothing
> in this guide changes the working directory. Download into
> `%USERPROFILE%\Downloads`:
>
> ```
> curl.exe -L -o "$env:USERPROFILE\Downloads\mujoco-3.15.0-windows-x86_64.zip" `
>     https://github.com/google-deepmind/mujoco/releases/download/3.15.0/mujoco-3.15.0-windows-x86_64.zip
> curl.exe -L -o "$env:USERPROFILE\Downloads\mujoco-3.15.0-windows-x86_64.zip.sha256" `
>     https://github.com/google-deepmind/mujoco/releases/download/3.15.0/mujoco-3.15.0-windows-x86_64.zip.sha256
> ```
>
> The zip is 22,142,419 bytes.

> ## 2. Verify the checksum
>
> The `.sha256` sidecar is a standard `sha256sum` line (hash + file
> name); it is the release page's independent statement of the artifact's
> digest. The published hash for the 3.15.0 Windows artifact is
> `b9cc1b50de60310a51110988388e080def5258b509f299682514774bccbc156a`
> (GitHub's release API reports the same digest for the asset). Verify the
> download against it without changing directories:
>
> ```
> if ((Get-FileHash "$env:USERPROFILE\Downloads\mujoco-3.15.0-windows-x86_64.zip" -Algorithm SHA256).Hash `
>     -eq 'b9cc1b50de60310a51110988388e080def5258b509f299682514774bccbc156a') { 'VERIFIED' }
> ```
>
> It prints `VERIFIED` on success and nothing otherwise. (`Get-FileHash`
> prints uppercase hex; `-eq` compares strings case-insensitively.) If it
> does not print, do not use the download; re-download or investigate the
> mirror.

> ## 3. Extract into the repo's dist
>
> Run from the repository root. Extract into
> `dist\mujoco\x86_64-windows\<version>` — a repo-local, gitignored prefix
> in the spirit of the OCCT dist (but separate: MuJoCo is a
> development/simulation dependency, never a Unity-package payload);
> `x86_64-windows` follows the repo's existing artifact naming. **Unlike
> the Linux tarball, the zip has no top-level directory** — `bin\`,
> `include\`, … are at the archive root — so it extracts straight into
> the version-named directory with nothing to strip:
>
> ```
> New-Item -ItemType Directory -Force dist\mujoco\x86_64-windows\3.15.0 | Out-Null
> Expand-Archive -Path "$env:USERPROFILE\Downloads\mujoco-3.15.0-windows-x86_64.zip" `
>     -DestinationPath dist\mujoco\x86_64-windows\3.15.0
> ```
>
> The zip stores its entry names with `\` separators
> (`bin\simulate.exe`); `Expand-Archive` turns those into directories.
>
> Installed layout (read back from the 3.15.0 zip's directory):
>
> ```
> dist\mujoco\x86_64-windows\3.15.0\
> ├── bin\                  simulate.exe, compile.exe, mujoco.dll, assets\ (runtime materials),
> │                         mujoco_plugin\*.dll, and the other samples
> ├── lib\                  mujoco.lib (import library)
> ├── include\mujoco\       the C API headers (72 files)
> ├── include\              also SDL2\, imgui/implot, math\, misc\
> ├── model\                the bundled model collection (humanoid, …)
> ├── sample\, simulate\    sample and simulate sources with their CMake setups
> ├── LICENSE               Apache-2.0
> └── THIRD_PARTY_NOTICES.txt
> ```
>
> `simulate.exe` runs in place: it imports `mujoco.dll`, which sits beside
> it in `bin\`, and Windows searches the folder the application loaded
> from before the system folders and `PATH` (Microsoft docs, *Dynamic-link
> library search order*) — nothing is installed system-wide, and the tree
> is relocatable provided `bin\` stays intact.

> ## 4. Validate
>
> Run the bundled viewer on a bundled model first — this proves the
> artifact's loader, rendering, and GUI work on your machine:
>
> ```
> dist\mujoco\x86_64-windows\3.15.0\bin\simulate.exe `
>     dist\mujoco\x86_64-windows\3.15.0\model\humanoid\humanoid.xml
> ```
>
> A window should open showing the humanoid model. MuJoCo writes
> warnings and errors to `MUJOCO_LOG.TXT` in the executable's directory,
> here `bin\` (harmless, deletable).
>
> Then the real target of this install — the Unity exporter's output, the
> M3 smoke test that was waiting on MuJoCo. The export lives beside the
> STEP file in the Unity project (`~` folder, Unity-ignored); for the
> cart-pole project on this machine:
>
> ```
> dist\mujoco\x86_64-windows\3.15.0\bin\simulate.exe `
>     'D:\git\cart-pole\unity\projects\cart-pole\Assets\asy-cart-pole.step~\asy-cart-pole.xml'
> ```
>
> The assembly must render and look like the STEP in a CAD viewer (the
> same visual-fidelity debugging rule as everywhere else in this repo: any
> mirroring is a bug in the exporter, never in the data). In `simulate`,
> open the Control panel and drive `03-Cart.Slider.Actuator` — the cart
> should slide along the rail and the pole should swing on its hinge while
> the welded parts stay rigid.
>
> **What this validation does not cover:** the numeric model checks
> (body/mesh/actuator counts, Σ `body_mass`) come from the Python
> bindings, which this guide deliberately does not install — pip is out
> of policy, and whether to build the bindings locally from the pinned
> source is a separate, open decision. Until that decision, `simulate` is
> the validation surface.

> ## Swapping versions
>
> Replace `3.15.0` in the step-1 URLs and the step-3 directory name, and
> the step-2 hash with the new release's `.sha256` sidecar value. Old
> version directories can be deleted; nothing else references them except
> the validating commands. Readthedocs "stable" tracks the latest
> release — when it moves past the pin, re-check `mujoco-schema.md` before
> bumping.

> ## Troubleshooting
>
> - **A missing-DLL error naming one of the four Prerequisites runtime
>   DLLs** — re-run that check; the runtime is not present.
> - **A missing-DLL error naming `mujoco.dll`** — `simulate.exe` was run
>   from outside its `bin\` (copied or moved without `mujoco.dll`); keep
>   the extracted tree intact and invoke `bin\simulate.exe` in place.
> - **Checksum mismatch** — re-download; compare the digest against the
>   value recorded in step 2. Never install an artifact that fails
>   verification.
> - **`MUJOCO_LOG.TXT` appears in `bin\`** — MuJoCo writes it there on
>   warnings/errors; it is a log, not an installation problem.

> ## Appendix: building from source (fallback only)
>
> Run from the repository root. Use this only when no release artifact
> fits the machine. Per `doc/programming` at the tag: CMake and a C++17
> compiler; with MSVC, Visual Studio 2019 or later and Windows SDK
> 10.0.22000 or later. The build auto-fetches its dependencies from
> upstream repositories over the network via CMake `FetchContent`, so git
> and network access are required at configure time.
>
> The commands run in a Visual Studio developer shell targeting x64,
> which puts the x64 `cl.exe` and Visual Studio's bundled `ninja.exe` on
> `PATH` (path shown for Visual Studio 2022 Community; adjust to the
> installed edition):
>
> ```
> & 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1' `
>     -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
> git clone --depth 1 --branch 3.15.0 https://github.com/google-deepmind/mujoco.git tmp\mujoco\mujoco
> cmake -S tmp\mujoco\mujoco -B tmp\mujoco\build -G Ninja `
>       -DCMAKE_BUILD_TYPE=Release `
>       "-DCMAKE_INSTALL_PREFIX=$PWD\dist\mujoco\x86_64-windows\3.15.0" `
>       -DMUJOCO_BUILD_TESTS=OFF
> cmake --build tmp\mujoco\build --parallel
> cmake --install tmp\mujoco\build
> ```
>
> Stock options, none modified: `-DMUJOCO_BUILD_TESTS=OFF` turns off the
> test build (default `ON`); `MUJOCO_BUILD_SIMULATE` is left at its
> default `ON` so the viewer is built. There is no local source modification, unlike the
> OCCT build. The install layout of a source build has not been read
> back; locate `simulate.exe` under the prefix before running step 4.

> ## References
>
> - MuJoCo documentation, *Programming*
>   (<https://mujoco.readthedocs.io/en/stable/programming.html>) — checked
>   against `doc/programming/index.rst` and `simulate/README.md` at tag
>   **3.15.0** (2026-10-22)
> - Release assets and checksums:
>   <https://github.com/google-deepmind/mujoco/releases/tag/3.15.0>
>   (`mujoco-3.15.0-windows-x86_64.zip.sha256` and the release API's asset
>   digest; the layout and PE import tables read from the zip's central
>   directory; all 2026-10-22)
> - Microsoft docs: *IsProcessorFeaturePresent*, *Dynamic-link library
>   search order*, *Loading an OpenGL Installable Client Driver*
> - Linux counterpart: `mujoco-linux.md`
> - Repo docs this install serves: `unity-mujoco.md` (M3 exporter),
>   `mujoco-schema.md` (schema facts)
