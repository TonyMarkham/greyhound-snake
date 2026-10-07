# Installing Open CASCADE Technology on macOS (build from release tag)

OCCT does **not** publish official prebuilt macOS binaries — the only mac asset
that ever appeared on GitHub releases was a 0-byte CI artifact
(`results-macos-x64.zip` in V8_0_0_p1). Official guidance for macOS is to build
from source and use Homebrew for third-party libraries. This doc builds the
latest stable release, **OCCT 8.0.1** (tag `V8.0.1`), from the release tag.

- Repo: <https://github.com/Open-Cascade-SAS/OCCT>
- Tag: <https://github.com/Open-Cascade-SAS/OCCT/releases/tag/V8.0.1>
- License: LGPL-2.1 with exception
- macOS binaries of third-party libraries are **not** attached to releases;
  use Homebrew (or build third-party libs yourself).

## Prerequisites

- macOS 11+ (Intel or Apple Silicon — arm64 is a first-class target in 8.0)
- Xcode Command Line Tools (Apple Clang 11+):

```
xcode-select --install
```

- [Homebrew](https://brew.sh), then the build toolchain and the required
  third-party libraries. OCCT's minimum is FreeType (visualization) and
  Tcl/Tk (DRAW test harness):

```
brew install cmake ninja tcl-tk freetype
```

Optional, only if you enable the matching `USE_*` flags later:

```
brew install tbb rapidjson draco freeimage      # USE_TBB, USE_RAPIDJSON, USE_DRACO, USE_FREEIMAGE
# vtk and ffmpeg are large; install only if you really need USE_VTK / USE_FFMPEG
```

## 1. Get the OCCT source at the release tag

Either clone the tag (shallow):

```
git clone --depth 1 --branch V8.0.1 https://github.com/Open-Cascade-SAS/OCCT.git
```

or download the tarball attached to the release:

```
curl -LO https://github.com/Open-Cascade-SAS/OCCT/archive/refs/tags/V8.0.1.tar.gz
tar xzf V8.0.1.tar.gz
```

(Sources are identical; the clone is ~45 MB compressed.)

## 2. Configure, build, install

Tcl/Tk in Homebrew is keg-only, so point CMake at it via `CMAKE_PREFIX_PATH`:

```
cmake -S OCCT -B build -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH="$(brew --prefix)/opt/tcl-tk"
cmake --build build --parallel
cmake --install build --prefix "$HOME/opt/occt-8.0.1"
```

Useful switches (add to the `cmake -S` line):

- `-DUSE_TBB=ON -DUSE_RAPIDJSON=ON -DUSE_DRACO=ON -DUSE_FREEIMAGE=ON` —
  enable optional third parties (install the matching Homebrew packages).
- `-DBUILD_MODULE_Draw=OFF` — skip the DRAW test harness (faster build;
  then Tcl/Tk is not required).
- `-DBUILD_LIBRARY_TYPE=Static` — static instead of shared libraries.

Build time: roughly 10–30 minutes depending on machine.

Installed layout (Unix layout):

```
~/opt/occt-8.0.1/
├── bin/        DRAWEXE, draw.sh, env.sh, custom.sh
├── lib/        libTK*.dylib, cmake/opencascade-8.0.1/
├── include/opencascade/
├── resources/  (Shaders, DrawResources, UnitsAPI, ...)
└── data/ doc/ tests/
```

## 3. Verify with DRAWEXE

```
"$HOME/opt/occt-8.0.1/draw.sh"
```

`draw.sh` sources `env.sh` next to it (sets `CASROOT`, resource paths,
`DYLD`/`PATH`) and starts DRAWEXE. Then:

```
pload ALL
box b 10 20 30
vinit
vdisplay b
vfit
```

A viewer window with a box should appear. On a headless/SSH session, use
`bprops b` (prints mass properties, no GUI). Type `exit` to leave.

## 4. Use OCCT in your CMake project

```
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH="$HOME/opt/occt-8.0.1"
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

The CMake config is installed at `lib/cmake/opencascade-8.0.1/`;
`CMAKE_PREFIX_PATH=$HOME/opt/occt-8.0.1` is resolved automatically.

## Swapping versions

Replace `V8.0.1` everywhere with another tag. Tag naming differs by era:
8.0.x uses `V8.0.1`/`V8_0_0` (dots and underscores both appear), 7.9.x uses
underscores (`V7_9_3`). The 8.0 series requires a C++17 compiler.

Platform docs (`overview-doc.zip`, `refman-doc.zip`) are attached to each
release and are platform-independent — download and unzip them if you want
offline documentation.

## Troubleshooting

- **Tcl/Tk not found during configure** — make sure
  `CMAKE_PREFIX_PATH` includes `$(brew --prefix)/opt/tcl-tk`.
- **FreeType not found** — `brew install freetype`; Homebrew's prefix is
  searched via `CMAKE_PREFIX_PATH`/default Homebrew linkage.
- **BUILD fails on C++ standard** — ensure CMake ≥ 3.10 (3.16+ recommended)
  and that `CMAKE_CXX_STANDARD` isn't forced below 17.
- **Viewer doesn't open over SSH** — DRAW's GUI needs a window server; use
  `bprops b`-style non-graphical checks instead.