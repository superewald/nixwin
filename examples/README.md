# Examples

Each example cross-compiles a small Windows program — touching the most common
base classes (kernel32, user32, gdi32, shell32, advapi32, ole32, oleaut32,
ws2_32) — against the nixwin sysroot using the integration files emitted by
`nixwin install`, and runs it with wine when available.

| directory | toolchain | deltas |
|---|---|---|
| [`llvm/`](llvm/README.md) | clang-cl + lld-link, driven directly | needs clang-cl and lld-link on `PATH` |
| [`cmake/`](cmake/README.md) | CMake with the generated `toolchain.cmake` | needs cmake and clang, and the lockfile-aware wrapper |
| [`rustc/`](rustc/README.md) | cargo with the `rustc.env` environment | needs cargo plus the `x86_64-pc-windows-msvc` target |

## Prerequisites

Shared by all three, and stated only here:

```sh
nixwin setup
nixwin install 17 --default
```

`nixwin setup` writes `NIXWIN_SYSROOT`, `NIXWIN_DATA` and `NIXWIN_CACHE` to your
shell rc, so open a new shell afterwards. `--default` makes sysroot `17` the one
`$NIXWIN_SYSROOT` points at, which is what the integration files are generated
for.

wine is optional: without it the executables are still linked into `build/` or
`target/` and only the build happens.

## Running one

```sh
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/llvm/build.sh
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/cmake/build.sh
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/rustc/build.sh
```

or, equivalently, from inside an example directory:

```sh
cd examples/llvm && ./build.sh
```

The `NIXWIN_SYSROOT` prefix is optional when the default sysroot lives at
`$HOME/.local/share/nixwin/sysroot`, because each `build.sh` falls back to that
path. It is required if you keep sysroots somewhere else, or if the default is
not `17`.

Each `build.sh` cross-compiles and then runs the binary under wine if it is
installed. The program prints a few lines describing the environment it ended up
in:

```
pid: 32
module: Z:\...\windows-example.exe
screen: 1920x1080
bpp: 32
argc: 1
windows: Windows 10 Pro
guid: 8f96781f-57c0-41fb
iid_iunknown: 00000000
bstr: nixwin
socket: 120
```

The `rustc` example prints one extra line, `cc pid: 32`, from the C++ file it
compiles through the `cc` crate.

## The same program, three times

`llvm/` and `cmake/` build the same `main.cpp`, so the difference between the two
is purely how the toolchain is configured. `rustc/` is a different program with
the same goal: it links the same set of Windows libraries, but through
`windows-sys` and a build script rather than a `CMakeLists.txt`.

That makes the three a useful comparison of what each tool needs from a sysroot:
`llvm.env` sets five variables and one flag string, `toolchain.cmake` sets
twenty CMake variables, and `rustc.env` sets seven variables including a
pre-computed `LIB` search path.
