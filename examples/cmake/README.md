# cmake example

Cross-compiles [`main.cpp`](main.cpp) with a stock
[`CMakeLists.txt`](CMakeLists.txt) that contains nothing nixwin-specific. All
the Windows configuration arrives through the generated
`toolchain.cmake`, which is what makes this the example to copy into an existing
CMake project.

Prerequisites beyond the [shared ones](../README.md#prerequisites):

- `cmake` 3.19 or newer
- `clang-cl` and `lld-link` (the toolchain file names them, it does not bundle
  them)

## Build

```sh
./build.sh
```

or, to see the individual steps:

```sh
source $NIXWIN_SYSROOT/cmake.env
cmake -B build
cmake --build build
```

## The generated toolchain

`cmake.env` points CMake at two files:

```sh
export CMAKE_TOOLCHAIN_FILE=$NIXWIN_SYSROOT/toolchain.cmake
export CMAKE_CLANG_VFS_OVERLAY=$NIXWIN_SYSROOT/vfsoverlay.json
```

`toolchain.cmake` is rendered per sysroot, with the CRT and SDK versions baked
in, and sets:

- `CMAKE_SYSTEM_NAME Windows`, so CMake looks for Windows-style libraries
- `clang-cl` as the C and C++ compiler, `lld-link` as the linker,
  `llvm-lib` as the archiver
- `--target=<triple>`, `/winsysroot` and the VFS overlay as the initial
  compiler flags
- `/libpath` entries for the MSVC CRT, the SDK `um` and the SDK `ucrt` of the
  selected architecture
- `CMAKE_FIND_ROOT_PATH` pointing at the sysroot, with
  `FIND_ROOT_PATH_MODE_PROGRAM NEVER` so host tools stay reachable

`TARGET_PLATFORM` selects the architecture, defaulting to the sysroot's primary
one:

```sh
cmake -B build -DTARGET_PLATFORM=Win32    # x86
cmake -B build -DTARGET_PLATFORM=Win64    # x86_64
```

## Lockfile detection

`toolchain.cmake` is bound to the sysroot it was generated in. To have CMake
choose the sysroot per project, run `nixwin setup --cmake` once: it writes a
*wrapper* to `$NIXWIN_DATA/toolchain.cmake` and exports it as
`CMAKE_TOOLCHAIN_FILE`.

The wrapper looks for a `.nixwin.json` in `CMAKE_CURRENT_SOURCE_DIR`, then in
its own directory, and includes `$NIXWIN_DATA/sysroots/$TAG/toolchain.cmake`
using the `tag` it finds there. With no lockfile it falls back to
`$NIXWIN_SYSROOT` and then `$NIXWIN_DATA/sysroot`.

So a committed [lockfile](../../docs/lockfiles.md) makes the sysroot a property
of the repository:

```sh
nixwin install 17 --lock .   # writes .nixwin.json naming tag "17"
nixwin setup --cmake         # one-time, machine level
cmake -B build               # picks sysroot 17 in every clone
```

## Notes

- The `CMakeLists.txt` links the common Windows import libraries by name
  (`kernel32`, `user32`, `gdi32`, `shell32`, `advapi32`, `ole32`, `oleaut32`,
  `uuid`, `ws2_32`) and nothing else — a normal CMake project cross-compiles
  once the toolchain file is in place.
- The wrapper re-resolves on every configure, so switching the default sysroot
  takes effect on the next `cmake -B build`. Add `--fresh` if CMake cached the
  toolchain variables in an existing build directory.
- See the [CMake guide](../../docs/cmake.md) for the full option list and the
  `tpl.toolchain` template override.
