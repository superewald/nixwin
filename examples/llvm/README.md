# llvm example

Cross-compiles [`main.cpp`](main.cpp) with `clang-cl` and links it with
`lld-link`, using the flags `llvm.env` exports. This is the lowest-level of the
three examples: nothing is generated at build time, the sysroot's environment
file is sourced and the driver is called directly.

Prerequisites beyond the [shared ones](../README.md#prerequisites) are on
`PATH`:

- `clang-cl` (clang 15 or newer)
- `lld-link`

## Build

```sh
./build.sh
```

or, to see the individual steps:

```sh
source $NIXWIN_SYSROOT/llvm.env
clang-cl $NIXWIN_LLVM_FLAGS main.cpp -o windows-example.exe
```

## What the integration does

`llvm.env` is generated into the sysroot at install time, so it always matches
the sysroot's path and its `vfsoverlay.json`:

```sh
export NIXWIN_SYSROOT="${NIXWIN_SYSROOT:-...}";
export CC=clang-cl
export CXX=clang-cl
export AR=llvm-lib
export NIXWIN_LLVM_FLAGS="-fuse-ld=lld-link /winsysroot $NIXWIN_SYSROOT -Xclang -ivfsoverlay -Xclang $NIXWIN_SYSROOT/vfsoverlay.json"
export CFLAGS=$NIXWIN_LLVM_FLAGS
export CXXFLAGS=$NIXWIN_LLVM_FLAGS
```

`$NIXWIN_LLVM_FLAGS` is the whole integration:

- `-fuse-ld=lld-link` makes the `clang-cl` driver link through `lld-link`
  instead of looking for a Windows `link.exe`.
- `/winsysroot $NIXWIN_SYSROOT` points the driver at the sysroot for headers
  and import libraries, so `kernel32`, `user32` and friends are found by their
  ordinary Windows names with no `/LIBPATH` of your own.
- `-Xclang -ivfsoverlay` applies the sysroot's `vfsoverlay.json`, which maps the
  case-insensitive header paths Windows code uses onto the case-sensitive
  directory names on a Linux or macOS host.

Because `CC`, `CXX`, `AR`, `CFLAGS` and `CXXFLAGS` are exported, a `Makefile` or
`meson.build` that honours them needs no other change:

```make
windows-example.exe: main.cpp
	$(CXX) $(CXXFLAGS) $< -o $@
```

## Notes

- The example links in release mode by default, so it works with a sysroot
  installed without `--features debug`. Pass `-g` to `clang-cl` for a debug
  build, which additionally needs the `debug` feature.
- There is no CMake or cargo involvement, so this is the example to reach for
  when diagnosing a sysroot problem: if `clang-cl` works here, the sysroot is
  sound and any failure elsewhere is in the tool's own configuration.
- `build.sh` runs the result under wine if it is installed. See the
  [wine guide](../../docs/wine.md) for the `WINEPATH` export, which a *debug*
  binary needs at run time.
