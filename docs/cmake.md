# CMake

Two files are emitted per sysroot: `toolchain.cmake`, a cross-compilation
toolchain, and `cmake.env`, which points CMake and the clang driver at it.
There is also a machine-level *wrapper* toolchain that picks the right sysroot
per project.

## Per-sysroot toolchain

The direct route. `cmake.env` sets the two variables `toolchain.cmake` needs:

```sh
source "$NIXWIN_SYSROOT/cmake.env"
cmake -B build
cmake --build build
```

```sh
export NIXWIN_SYSROOT="${NIXWIN_SYSROOT:-/home/you/.local/share/nixwin/sysroot}";
export CMAKE_TOOLCHAIN_FILE=$NIXWIN_SYSROOT/toolchain.cmake
export CMAKE_CLANG_VFS_OVERLAY=$NIXWIN_SYSROOT/vfsoverlay.json
```

Equivalently, without sourcing anything:

```sh
cmake -B build \
  -DCMAKE_TOOLCHAIN_FILE="$NIXWIN_SYSROOT/toolchain.cmake" \
  -DCMAKE_CLANG_VFS_OVERLAY="$NIXWIN_SYSROOT/vfsoverlay.json"
```

## What the toolchain configures

| setting | value |
|---|---|
| `CMAKE_SYSTEM_NAME` | `Windows` |
| `CMAKE_C_COMPILER` / `CMAKE_CXX_COMPILER` | `clang-cl` |
| `CMAKE_AR` | `llvm-lib` |
| `CMAKE_LINKER` | `lld-link` |
| `CMAKE_C_FLAGS_INIT` / `CMAKE_CXX_FLAGS_INIT` | `--target=<triple>`, `/winsysroot`, the VFS overlay |
| `CMAKE_*_LINKER_FLAGS_INIT` | the VFS overlay plus `/libpath` for the CRT, `um` and `ucrt` of the selected architecture |
| `CMAKE_FIND_ROOT_PATH` | the sysroot, with `FIND_ROOT_PATH_MODE_PROGRAM NEVER` so host tools are still found |

The target architecture defaults to the primary architecture of the sysroot and
is overridable:

```sh
cmake -B build -DTARGET_PLATFORM=Win32    # x86
cmake -B build -DTARGET_PLATFORM=Win64    # x86_64
```

`TARGET_PLATFORM` maps onto both the compiler triple
(`i686-pc-windows-msvc`, `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`,
`armv7-pc-windows-msvc`) and the MSVC library directory name (`x86`, `x64`,
`arm64`, `arm`), so the two cannot drift apart.

## The lockfile-aware wrapper

Per-sysroot `toolchain.cmake` is bound to the sysroot it was generated in.
`nixwin setup --cmake` instead writes a single wrapper to
`$NIXWIN_DATA/toolchain.cmake` and exports it as `CMAKE_TOOLCHAIN_FILE`, so one
shell configuration serves every project:

```sh
nixwin setup --cmake
```

The wrapper looks for a `.nixwin.json` in `CMAKE_CURRENT_SOURCE_DIR` and then in
its own directory. If it finds one, it reads the `tag` from it and includes
`$NIXWIN_DATA/sysroots/$TAG/toolchain.cmake` — the sysroot the project pinned,
not whichever one happens to be the default. With no lockfile it falls back to
`$NIXWIN_SYSROOT` and then to `$NIXWIN_DATA/sysroot`.

That is what makes a committed [lockfile](lockfiles.md) work end to end: a
`cmake -B build` in a CI job picks the same sysroot on every machine without a
flag, because the tag is in the repository.

The wrapper re-resolves on every configure, so switching the default sysroot
takes effect on the next `cmake -B build`; add `--fresh` if the toolchain
variables are cached in an existing build directory.

!!! note "The block is rewritten on every `nixwin setup` run"

    `CMAKE_TOOLCHAIN_FILE` lives in its own managed block,
    `# >>> nixwin (cmake) >>>`. Put project-specific CMake arguments in
    `CMakePresets.json` or a `CMakeLists.txt` rather than editing that block.

## A worked example

`examples/cmake` is a complete project: a `CMakeLists.txt` that links the
common Windows import libraries and nothing nixwin-specific.

```sh
cd examples/cmake
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot ./build.sh
```

See [`examples/cmake/README.md`](https://github.com/superewald/nixwin/blob/main/examples/cmake/README.md)
for the details.
