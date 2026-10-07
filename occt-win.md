# Installing Open CASCADE Technology on Windows (release binaries)

OCCT ships official prebuilt Windows binaries (MSVC x64) attached to every GitHub
release since V7_8_0. This doc installs the latest stable release,
**OCCT 8.0.1** (tag `V8.0.1`), using those binaries — no compilation required.

- Repo: <https://github.com/Open-Cascade-SAS/OCCT>
- Release: <https://github.com/Open-Cascade-SAS/OCCT/releases/tag/V8.0.1>
- License: LGPL-2.1 with exception (`LICENSE_LGPL_21.txt`, `OCCT_LGPL_EXCEPTION.txt` in every package)

> **Note:** binaries exist **only for Windows (MSVC x64)**. For macOS and Linux,
> see `occt-mac.md` / `occt-linux.md` (build from the same release tag).

## Prerequisites

- Windows 10/11, 64-bit
- Visual Studio 2019 or 2022 with the **Desktop development with C++** workload.
  The binaries are built with the `vc14` ABI (VS 2015–2022 compatible runtime);
  official CI uses VS 2022.
- CMake 3.10+ if you will consume OCCT from CMake projects (bundled with VS 2022).

## 1. Download the release assets

From <https://github.com/Open-Cascade-SAS/OCCT/releases/latest> (V8.0.1), download **one** of:

| Asset | Size | Contents |
|---|---|---|
| `occt-combined-release-pch.zip` | 246 MB | OCCT (Release) **+ all 3rd-party libs** — recommended single download |
| `opencascade-release-pch.zip` + `3rdparty-vc14-64.zip` | 52 MB + 180 MB | OCCT (Release) and 3rd-party libs as separate packages |
| `occt-combined-with-debug-pch.zip` | 455 MB | OCCT Release + Debug + all 3rd-party libs (only if you need Debug) |

Naming scheme:

- `occt-combined-*` — OCCT + the `3rdparty-vc14-64` bundle in one package.
- `*-with-debug-*` — additionally includes Debug libraries (link your Debug builds against these).
- `*pch*` vs `*no-pch*` — whether OCCT itself was compiled with precompiled headers.
  No effect on consumers; pick either.
- Optional: `overview-doc.zip` (17 MB) and `refman-doc.zip` (107 MB) — HTML docs, platform-independent.

Checksums (sha256, from the release page):

```
2acafd123f1cb8b0fc7f1f7f65ff12f468b6a675acf8b2b00a4b5bcf61ebcaef  occt-combined-release-pch.zip
5dfee6ea32c22b2f2e63722309f27b356c4dffe1f6a89d3b589f67f13a6564af  opencascade-release-pch.zip
12d04f688c583b8e896758f65cd7012458fee921cedb792ec6caaf469b372598  3rdparty-vc14-64.zip
```

## 2. Extract (twice)

Each asset is **a zip containing a single inner zip** (CI packaging quirk):

```
occt-combined-release-pch.zip
└── opencascade-8.0.1-vc14-64-pch-combined.zip   (extract this too)
    ├── opencascade-8.0.1-vc14-64/               (OCCT itself)
    └── 3rdparty-vc14-64/                        (tcltk, freetype, tbb, qt, vtk, ...)
```

Extract the downloaded zip, then extract the inner zip inside it. Recommended
location: `C:\OpenCASCADE\` (short path, avoids long-path issues).

Final layout (combined variant):

```
C:\OpenCASCADE\
├── opencascade-8.0.1-vc14-64\
│   ├── env.bat  draw.bat  custom.bat  custom_vc14_64.bat
│   ├── win64\vc14\bin\    (TK*.dll, DRAWEXE.exe)
│   ├── win64\vc14\lib\    (TK*.lib import libraries)
│   ├── inc\               (headers)
│   ├── cmake\             (OpenCASCADEConfig.cmake)
│   ├── src\               (runtime resources: Shaders, DrawResources, UnitsAPI, ...)
│   └── data\              (sample brep/iges/step/stl files)
└── 3rdparty-vc14-64\
    ├── freetype-2.13.3-x64\  tbb-2021.13.0-x64\  tcltk-8.6.15-x64\
    ├── freeimage-3.18.0-x64\ vtk-9.4.1-x64\      qt5.11.2-vc14-64\
    └── angle-gles2, draco, ffmpeg, gl2ps, glfw, jemalloc, lzma, openvr, rapidjson, zlib, ...
```

> Keep the two folders **side by side as siblings** — `env.bat` defaults to
> `THIRDPARTY_DIR=..\3rdparty-vc14-64` (sibling of the OCCT root).

If you downloaded the non-combined pair, extract `opencascade-release-pch.zip`
(the same double-extraction) and place `3rdparty-vc14-64\` next to the
`opencascade-8.0.1-vc14-64\` folder exactly as above.

## 3. Verify with DRAWEXE

Open a **VS developer prompt** ("x64 Native Tools Command Prompt for VS 2022"):

```
cd C:\OpenCASCADE\opencascade-8.0.1-vc14-64
draw.bat
```

`draw.bat` calls `env.bat` (sets `CASROOT`, `CSF_*` resource paths, prepends
third-party DLL dirs to `PATH`) and starts `DRAWEXE.exe`. Then:

```
pload ALL
box b 10 20 30
vinit
vdisplay b
vfit
```

A viewer window with a box should appear. For a non-graphical check, use
`bprops b` instead of the `v*` commands. Type `exit` to leave.

## 4. Use OCCT in your CMake project

The package ships CMake config files in `cmake\` (Windows-style prefix layout),
so pointing `CMAKE_PREFIX_PATH` at the OCCT root is enough:

```cmake
cmake -S . -B build -G "Visual Studio 17 2022" -A x64 ^
      -DCMAKE_PREFIX_PATH="C:/OpenCASCADE/opencascade-8.0.1-vc14-64" ^
      -DCMAKE_BUILD_TYPE=Release
```

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.16)
project(hello_occt CXX)
set(CMAKE_CXX_STANDARD 17)

find_package(OpenCASCADE REQUIRED)

add_executable(hello main.cpp)
target_include_directories(hello PRIVATE ${OpenCASCADE_INCLUDE_DIR})
# Imported targets are un-namespaced: TKernel, TKBRep, TKTopAlgo, TKDESTEP, ...
target_link_libraries(hello PRIVATE TKernel TKBRep TKTopAlgo TKDESTEP)
```

Useful variables set by `find_package(OpenCASCADE)`:

- `OpenCASCADE_INCLUDE_DIR` — `inc\`
- `OpenCASCADE_LIBRARY_DIR` — `win64\vc14\lib`
- `OpenCASCADE_BINARY_DIR` — `win64\vc14\bin`
- `OpenCASCADE_LIBRARIES` — every toolkit (link only what you need instead;
  per-module sets like `OpenCASCADE_DataExchange_LIBRARIES` also exist)

Notes:

- The Release-only zips contain Release libraries — build your app in `Release`.
  Use a `*-with-debug-*` package if you need to link Debug builds.
- To run your app, `TK*.dll` plus third-party DLLs (e.g. `freetype.dll`,
  `tbb12.dll`) must be on `PATH` or next to your exe — copy them from
  `win64\vc14\bin` and `3rdparty-vc14-64\*\bin`.

## Swapping versions

Replace `V8.0.1` in download URLs and folder names with any other tag.
Tag naming differs by era:

- 8.0.x: `V8.0.1`, `V8_0_0`, `V8_0_0_p1` (dots and underscores both appear)
- 7.9.x: `V7_9_3`, `V7_9_2`, ... (assets named like `opencascade-7.9.3-vc14-64-pch.zip`)
- 7.8.x: `V7_8_0` (assets named like `occt-vc143-64.zip`)

The 8.0 series requires a C++17 compiler; its ABI is stable since `V8_0_0_p1`.

## Troubleshooting

- **"DLL not found"** when launching DRAWEXE or your app — start from
  `draw.bat`/`env.bat`, or add `win64\vc14\bin` and the relevant
  `3rdparty-vc14-64\*\bin` dirs to `PATH`.
- **Linker errors about runtime mismatch** — the binaries use the `vc14`
  ABI (VS 2015–2022 compatible); don't mix with MinGW or static CRT configs.
- **Empty extraction result** — you extracted only the outer zip; extract the
  inner zip file as well.