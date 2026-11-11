# Releasing the native libraries

The Unity packages download their native libraries at editor load
instead of shipping them. OCCT and OpenCV come from the `occt-dev` and
`opencv-dev` releases. Our own three libraries come from a
`greyhound-snake` release, published by this procedure:

| Library | Built by | Package folder it downloads into | Config group |
|---|---|---|---|
| host (`importer_host`) | `cargo build -p importer-host` → `target/release/` | `com.greyhound.step/Runtime/Plugins/<arch>` | `greyhound_host` |
| OCCT shim (`greyhound_occt`) | `occt-sys` build.rs → its `shim_dir` | `com.greyhound.step/Runtime/Plugins/shim/<arch>` | `greyhound_shim` |
| OpenCV shim (`ghcv`) | `opencv-sys` build.rs → its `shim_dir` | `com.greyhound.vision/Runtime/Plugins/shim/<arch>` | `greyhound_ghcv` |

File names and `<arch>` per platform (`<arch>` is
`NativeMethods.PluginArch()`; the shim names come from the `-sys` crates'
`config.toml`):

| Platform | `<arch>` | host | OCCT shim | OpenCV shim |
|---|---|---|---|---|
| Windows | `x86_64` | `importer_host.dll` | `greyhound_occt.dll` | `ghcv.dll` |
| Linux | `x86_64` | `libimporter_host.so` | `libgreyhound_occt.so` | `libghcv.so` |
| macOS | `arm64` | `libimporter_host.dylib` | `libgreyhound_occt.dylib` | `libghcv.dylib` |

A platform with no entry in the config downloads nothing and uses
whatever a local build put in the folder.

## How the provisioner decides (rules you can check by hand)

For each folder, at editor load:

| Folder state | Result |
|---|---|
| No config entry for this platform | skipped |
| `<folder>.provenance.toml` matches the pinned url + sha256 | skipped |
| No marker, but the folder holds a non-`.meta` file | skipped (a local cargo build is never overwritten) |
| Anything else (empty/missing folder, or a marker for another pin) | download, verify sha256, replace folder, write marker |

The rule that matters on a dev machine is the last one. After you bump
a pin, a folder with a marker for the old pin is re-downloaded,
overwriting any local build you put there afterwards. To keep a local
build after a bump, delete that folder's `.provenance.toml`.

## One release, every platform

A version has one release, tagged `v<version>`, and every platform's
zips are assets on it. Each platform builds on its own machine. The
release is created once, as an empty draft ("Create the draft release"
below); every platform then uploads its own zips to it (step 4), and
step 5 publishes it once all of them are there. Zip names carry the
platform (`-x86_64-windows`, `-x86_64-linux`, `-arm64-macos`), so the
assets never collide.

Once a release is published, never replace its assets. Anyone who has
already downloaded them has a marker pinned to the old hash. Any change
is a new version.

The release version is the workspace version in the root `Cargo.toml`
(`[workspace.package] version`), which every crate inherits.

> ## Create the draft release
>
> Run this ONCE per version, from any one machine, before any platform
> uploads. Running it a second time doesn't fail: it makes a second,
> separate draft. `--draft` keeps the release private; nothing on it is
> downloadable until step 5 publishes it.
>
>> ### Windows (PowerShell)
>>
>> ```powershell
>> $version = ((cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages | Where-Object name -eq 'importer-host').version
>> ```
>> ```powershell
>> gh release create "v$version" --draft --title "v$version" --notes "Native libraries for the Unity packages; see RELEASE.md." --repo TonyMarkham/greyhound-snake
>> ```
>
>> ### Linux and macOS (bash)
>>
>> ```bash
>> version=$(cargo pkgid -p importer-host)
>> ```
>> ```bash
>> version=${version##*[#@]}
>> ```
>> ```bash
>> gh release create "v$version" --draft --title "v$version" --notes "Native libraries for the Unity packages; see RELEASE.md." --repo TonyMarkham/greyhound-snake
>> ```
>

> ## Windows
>
>> ### 0. Fetch the build inputs (once per clone)
>>
>> The shims compile against the OCCT and OpenCV dev trees under
>> `extern/`, which is gitignored, so a fresh clone has neither. Without
>> them the build fails with `missing OCCT headers` / `missing OpenCV
>> headers`.
>>
>> What each command does: it runs this repo's `extern-fetch` tool, which
>> reads `crates/extern-fetch/config.toml`, takes the group's `dev` entry
>> for the OS it runs on (`[occt.windows.dev]`, `[opencv.windows.dev]`),
>> downloads that zip from its GitHub release, checks its SHA-256 against
>> the pinned `sha256` (a mismatch stops before extracting), unpacks it
>> into the entry's `dest` (`extern/occt/8.0.1/win64-dev`,
>> `extern/opencv/5.0.0/win64-dev`), and writes `<dest>.provenance.toml`
>> beside it so a re-run is a no-op. It skips `payload` entries, so it
>> never touches the packages.
>>
>> ```bash
>> cargo run -p extern-fetch -- --group occt
>> ```
>> ```bash
>> cargo run -p extern-fetch -- --group opencv
>> ```
>
>> ### 1. Build
>>
>> Skip this when `cargo build` reports nothing to rebuild. Close every
>> Unity editor that uses `dist/package` first: Windows refuses to
>> overwrite a DLL a running editor has loaded, and the shim link fails
>> with `LNK1104`.
>>
>> ```bash
>> cargo build --release -p importer-host -p opencv-sys
>> ```
>
>> ### 2. Zip
>>
>> Each zip holds its one DLL at the zip root, because the provisioner
>> extracts a zip straight into its target folder.
>>
>> Read the version once into `$version`, in the same PowerShell session
>> you run steps 2–4 in:
>>
>> ```powershell
>> $version = ((cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages | Where-Object name -eq 'importer-host').version
>> ```
>>
>> `$version` now holds, e.g., `0.1.0`. The tag is `v$version`.
>>
>> ```powershell
>> New-Item -ItemType Directory -Force dist/release
>> ```
>> ```powershell
>> Compress-Archive -Force -Path target/release/importer_host.dll -DestinationPath "dist/release/importer_host-$version-x86_64-windows.zip"
>> ```
>> ```powershell
>> Compress-Archive -Force -Path dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/greyhound_occt.dll -DestinationPath "dist/release/greyhound_occt-$version-x86_64-windows.zip"
>> ```
>> ```powershell
>> Compress-Archive -Force -Path dist/package/com.greyhound.vision/Runtime/Plugins/shim/x86_64/ghcv.dll -DestinationPath "dist/release/ghcv-$version-x86_64-windows.zip"
>> ```
>
>> ### 3. Hash
>>
>> ```powershell
>> Get-FileHash dist/release/*.zip -Algorithm SHA256
>> ```
>>
>> PowerShell prints the hashes in uppercase. Paste them lowercase or
>> uppercase: both the config validator and the provisioner accept
>> either.
>
>> ### 4. Upload
>>
>> Upload this platform's zips to the draft release (created once per
>> version, in "Create the draft release" above):
>>
>> ```powershell
>> gh release upload "v$version" "dist/release/importer_host-$version-x86_64-windows.zip" "dist/release/greyhound_occt-$version-x86_64-windows.zip" "dist/release/ghcv-$version-x86_64-windows.zip" --repo TonyMarkham/greyhound-snake
>> ```
>

> ## Linux
>
> Run on the Linux x86_64 machine, in bash, from the repo root. The zip
> commands need the `zip` tool.
>
> WSL2 works as that machine. Use a separate clone inside WSL (e.g.
> `~/git/greyhound-snake`), not the Windows checkout under `/mnt/d`. Both
> builds write `target/release/` and would overwrite each other's cached
> build state. A branch that exists only in the Windows checkout comes
> over with `git fetch /mnt/d/git/greyhound-snake <branch>:<branch>`.
>
>> ### 0. Fetch the build inputs (once per clone)
>>
>> The shims compile against the OCCT and OpenCV dev trees under
>> `extern/` (here `extern/occt/8.0.1/linux-x86_64-dev` and
>> `extern/opencv/5.0.0/linux-x86_64-dev`), which is gitignored, so a
>> fresh clone has neither. Without them the build fails with
>> `missing OCCT headers` / `missing OpenCV headers`.
>>
>> What each command does: it runs this repo's `extern-fetch` tool, which
>> reads `crates/extern-fetch/config.toml`, takes the group's `dev` entry
>> for the OS it runs on (`[occt.linux.dev]`, `[opencv.linux.dev]`),
>> downloads that zip from its GitHub release, checks its SHA-256 against
>> the pinned `sha256` (a mismatch stops before extracting), unpacks it
>> into the entry's `dest`, and writes `<dest>.provenance.toml` beside it
>> so a re-run is a no-op. It skips `payload` entries, so it never touches
>> the packages.
>>
>> ```bash
>> cargo run -p extern-fetch -- --group occt
>> ```
>> ```bash
>> cargo run -p extern-fetch -- --group opencv
>> ```
>
>> ### 1. Build
>>
>> Skip this when `cargo build` reports nothing to rebuild.
>>
>> ```bash
>> cargo build --release -p importer-host -p opencv-sys
>> ```
>
>> ### 2. Zip
>>
>> Each zip holds its one library at the zip root (`zip -j` drops the
>> directory part), because the provisioner extracts a zip straight into
>> its target folder.
>>
>> Read the version once into `$version`, in the same shell you run steps
>> 2–4 in. `cargo pkgid` prints e.g.
>> `path+file:///…/crates/importer-host#0.1.0`; the second line keeps what
>> follows the last `#` or `@`:
>>
>> ```bash
>> version=$(cargo pkgid -p importer-host)
>> ```
>> ```bash
>> version=${version##*[#@]}
>> ```
>>
>> `$version` now holds, e.g., `0.1.0`. The tag is `v$version`.
>>
>> ```bash
>> mkdir -p dist/release
>> ```
>> ```bash
>> zip -j "dist/release/importer_host-$version-x86_64-linux.zip" target/release/libimporter_host.so
>> ```
>> ```bash
>> zip -j "dist/release/greyhound_occt-$version-x86_64-linux.zip" dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64/libgreyhound_occt.so
>> ```
>> ```bash
>> zip -j "dist/release/ghcv-$version-x86_64-linux.zip" dist/package/com.greyhound.vision/Runtime/Plugins/shim/x86_64/libghcv.so
>> ```
>>
>> `zip` adds to an existing archive instead of replacing it, so delete
>> an old zip of the same name before re-running a line.
>
>> ### 3. Hash
>>
>> ```bash
>> sha256sum dist/release/*-x86_64-linux.zip
>> ```
>
>> ### 4. Upload
>>
>> Upload this platform's zips to the draft release (created once per
>> version, in "Create the draft release" above):
>>
>> ```bash
>> gh release upload "v$version" "dist/release/importer_host-$version-x86_64-linux.zip" "dist/release/greyhound_occt-$version-x86_64-linux.zip" "dist/release/ghcv-$version-x86_64-linux.zip" --repo TonyMarkham/greyhound-snake
>> ```
>

> ## macOS
>
> Run on the Apple Silicon (arm64) Mac, in zsh or bash, from the repo
> root. Unity's Apple Silicon editor loads plugins from `arm64` folders,
> never `x86_64`.
>
>> ### 0. Fetch the build inputs (once per clone)
>>
>> The shims compile against the OCCT and OpenCV dev trees under
>> `extern/` (here `extern/occt/8.0.1/macos-arm64-dev` and
>> `extern/opencv/5.0.0/macos-arm64-dev`), which is gitignored, so a
>> fresh clone has neither. Without them the build fails with
>> `missing OCCT headers` / `missing OpenCV headers`.
>>
>> What each command does: it runs this repo's `extern-fetch` tool, which
>> reads `crates/extern-fetch/config.toml`, takes the group's `dev` entry
>> for the OS it runs on (`[occt.macos.dev]`, `[opencv.macos.dev]`),
>> downloads that zip from its GitHub release, checks its SHA-256 against
>> the pinned `sha256` (a mismatch stops before extracting), unpacks it
>> into the entry's `dest`, and writes `<dest>.provenance.toml` beside it
>> so a re-run is a no-op. It skips `payload` entries, so it never touches
>> the packages.
>>
>> ```bash
>> cargo run -p extern-fetch -- --group occt
>> ```
>> ```bash
>> cargo run -p extern-fetch -- --group opencv
>> ```
>
>> ### 1. Build
>>
>> Skip this when `cargo build` reports nothing to rebuild.
>>
>> ```bash
>> cargo build --release -p importer-host -p opencv-sys
>> ```
>
>> ### 2. Zip
>>
>> Each zip holds its one library at the zip root (`zip -j` drops the
>> directory part), because the provisioner extracts a zip straight into
>> its target folder.
>>
>> Read the version once into `$version`, in the same shell you run steps
>> 2–4 in (same two lines as Linux):
>>
>> ```bash
>> version=$(cargo pkgid -p importer-host)
>> ```
>> ```bash
>> version=${version##*[#@]}
>> ```
>>
>> `$version` now holds, e.g., `0.1.0`. The tag is `v$version`.
>>
>> ```bash
>> mkdir -p dist/release
>> ```
>> ```bash
>> zip -j "dist/release/importer_host-$version-arm64-macos.zip" target/release/libimporter_host.dylib
>> ```
>> ```bash
>> zip -j "dist/release/greyhound_occt-$version-arm64-macos.zip" dist/package/com.greyhound.step/Runtime/Plugins/shim/arm64/libgreyhound_occt.dylib
>> ```
>> ```bash
>> zip -j "dist/release/ghcv-$version-arm64-macos.zip" dist/package/com.greyhound.vision/Runtime/Plugins/shim/arm64/libghcv.dylib
>> ```
>>
>> `zip` adds to an existing archive instead of replacing it, so delete
>> an old zip of the same name before re-running a line.
>
>> ### 3. Hash
>>
>> ```bash
>> shasum -a 256 dist/release/*-arm64-macos.zip
>> ```
>
>> ### 4. Upload
>>
>> Upload this platform's zips to the draft release (created once per
>> version, in "Create the draft release" above):
>>
>> ```bash
>> gh release upload "v$version" "dist/release/importer_host-$version-arm64-macos.zip" "dist/release/greyhound_occt-$version-arm64-macos.zip" "dist/release/ghcv-$version-arm64-macos.zip" --repo TonyMarkham/greyhound-snake
>> ```
>

> ## 5. Publish the release
>
> Run once, from any platform, after every platform's zips are on the
> draft. From here on the assets are public and must never be replaced.
> `$version` is the one set in "Create the draft release" or step 2.
>
>> ### Check the assets
>>
>> Three zips per released platform:
>>
>> ```bash
>> gh release view "v$version" --repo TonyMarkham/greyhound-snake
>> ```
>
>> ### Publish
>>
>> ```bash
>> gh release edit "v$version" --draft=false --repo TonyMarkham/greyhound-snake
>> ```
>

> ## 6. Pin
>
> Add or update these groups in `crates/extern-fetch/config.toml`, with
> one `.payload` section per released platform. Every `url` must start
> with `https://github.com/<repo>/releases/download/<tag>/` (same-tag
> invariant, enforced by both extern-fetch and the provisioner), so `tag`
> and every `url` change together on a version bump. TOML can't read
> `$version`, so in the entries below, replace every `<version>` with the
> Cargo.toml version (`0.1.0` → `tag = "v0.1.0"`,
> `.../v0.1.0/importer_host-0.1.0-x86_64-windows.zip`). Leave out the
> section of a platform that hasn't been released; that platform then
> keeps its local build.
>
> ```toml
> [greyhound_host]
> repo = "TonyMarkham/greyhound-snake"
> tag  = "v<version>"
>
> [greyhound_host.windows.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/importer_host-<version>-x86_64-windows.zip"
> sha256 = "<importer_host windows zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/x86_64"
>
> [greyhound_host.linux.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/importer_host-<version>-x86_64-linux.zip"
> sha256 = "<importer_host linux zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/x86_64"
>
> [greyhound_host.macos.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/importer_host-<version>-arm64-macos.zip"
> sha256 = "<importer_host macos zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/arm64"
>
> [greyhound_shim]
> repo = "TonyMarkham/greyhound-snake"
> tag  = "v<version>"
>
> [greyhound_shim.windows.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/greyhound_occt-<version>-x86_64-windows.zip"
> sha256 = "<greyhound_occt windows zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64"
>
> [greyhound_shim.linux.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/greyhound_occt-<version>-x86_64-linux.zip"
> sha256 = "<greyhound_occt linux zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/shim/x86_64"
>
> [greyhound_shim.macos.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/greyhound_occt-<version>-arm64-macos.zip"
> sha256 = "<greyhound_occt macos zip hash>"
> dest   = "dist/package/com.greyhound.step/Runtime/Plugins/shim/arm64"
>
> [greyhound_ghcv]
> repo = "TonyMarkham/greyhound-snake"
> tag  = "v<version>"
>
> [greyhound_ghcv.windows.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/ghcv-<version>-x86_64-windows.zip"
> sha256 = "<ghcv windows zip hash>"
> dest   = "dist/package/com.greyhound.vision/Runtime/Plugins/shim/x86_64"
>
> [greyhound_ghcv.linux.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/ghcv-<version>-x86_64-linux.zip"
> sha256 = "<ghcv linux zip hash>"
> dest   = "dist/package/com.greyhound.vision/Runtime/Plugins/shim/x86_64"
>
> [greyhound_ghcv.macos.payload]
> url    = "https://github.com/TonyMarkham/greyhound-snake/releases/download/v<version>/ghcv-<version>-arm64-macos.zip"
> sha256 = "<ghcv macos zip hash>"
> dest   = "dist/package/com.greyhound.vision/Runtime/Plugins/shim/arm64"
> ```
>
> The role is `payload`, so the extern-fetch CLI skips these entries. Only
> the packages' C# provisioners write these folders.
>

> ## 7. Copy the config into the packages
>
> The provisioners read the tracked copies
> `package/com.greyhound.step/Editor/occt-fetch.toml` and
> `package/com.greyhound.vision/Editor/opencv-fetch.toml`, not the repo
> file. Until they're re-copied, the packages keep the old pins, and a
> missing `greyhound_*` entry silently skips that library's download. On
> any platform:
>
> ```bash
> just sync-fetch-config
> ```
>
> Commit the two copies together with `crates/extern-fetch/config.toml`.
>

> ## 8. Verify the download
>
> With every Unity editor closed, delete the platform's three libraries
> from the `package/com.greyhound.*` folders in the tables above (they're
> gitignored; leave the `.meta` files). An empty folder is the "download"
> case. Open a project that uses `package/`, and
> check that the Console shows (Windows paths shown; Linux uses
> `/Runtime/Plugins/x86_64`, macOS `/Runtime/Plugins/arm64`):
>
> ```
> [com.greyhound.step] greyhound_host payload provisioned: 1 files into …\Runtime\Plugins\x86_64
> [com.greyhound.step] greyhound_shim payload provisioned: 1 files into …\Runtime\Plugins\shim\x86_64
> [com.greyhound.vision] greyhound_ghcv payload provisioned: 1 files into …\Runtime\Plugins\shim\x86_64
> ```
>
> Then check that a STEP import works. Each folder now has a sibling
> `.provenance.toml` naming its group, url and sha256. On the next editor
> load, nothing downloads.
>
