# clang-cl and lld-link

`llvm.env` configures the LLVM toolchain for a sysroot. Source it and then call
`clang-cl` as you would on Windows; the sysroot, the linker and the
case-insensitivity workaround are already set up.

```sh
source "$NIXWIN_SYSROOT/llvm.env"
clang-cl main.cpp -o app.exe
```

## What it sets

```sh
export NIXWIN_SYSROOT="${NIXWIN_SYSROOT:-/home/you/.local/share/nixwin/sysroot}";
export CC=clang-cl
export CXX=clang-cl
export AR=llvm-lib
export NIXWIN_LLVM_FLAGS="-fuse-ld=lld-link /winsysroot $NIXWIN_SYSROOT -Xclang -ivfsoverlay -Xclang $NIXWIN_SYSROOT/vfsoverlay.json"
export CFLAGS=$NIXWIN_LLVM_FLAGS
export CXXFLAGS=$NIXWIN_LLVM_FLAGS
```

| export | why |
|---|---|
| `CC` / `CXX` | `clang-cl` is the MSVC-compatible driver, so it takes the `/`-style flags these builds use |
| `AR` | `llvm-lib` produces Windows `.lib` archives |
| `-fuse-ld=lld-link` | links through `lld-link` instead of trying to find a Windows link.exe |
| `/winsysroot $NIXWIN_SYSROOT` | points the driver at the sysroot for headers and import libraries |
| `-Xclang -ivfsoverlay` | applies the sysroot's `vfsoverlay.json`, so case-insensitive Windows header includes resolve on a case-sensitive host |
| `CFLAGS` / `CXXFLAGS` | the same flags for build systems which only read the standard variables |

`NIXWIN_SYSROOT` is exported with a `${NIXWIN_SYSROOT:-…}` default, so
sourcing the file is safe whether or not `nixwin setup` has already run.

## Driving a build system

Because `CFLAGS`, `CXXFLAGS`, `CC`, `CXX` and `AR` are exported, a build system
which honours them needs nothing else. A `Makefile` that calls `$(CC)` and
`$(CXX)`:

```make
app.exe: main.cpp
	$(CXX) $(CXXFLAGS) $< -o $@
```

Or pass the flags explicitly when the build system does not read the
environment:

```sh
clang-cl $NIXWIN_LLVM_FLAGS main.cpp -o app.exe
```

Import libraries are found through `/winsysroot`, so link against them by their
usual Windows names (`kernel32`, `user32`, `ws2_32`, …) without adding
`/LIBPATH` yourself.

## Debug builds

Debug libraries come from the `debug` feature of the install. Without it,
`clang-cl` compiles fine but linking a debug build fails on missing `libvcruntime*_d`/`libucrt*d`.

```sh
nixwin install 17 --features debug --default
```

To actually *run* a debug binary, wine needs the debug CRT DLLs too — see
[wine](wine.md).

## No environment file?

Every sysroot also has a plain `toolchain.cmake` and per-tool environment files.
If you would rather not source shell code, [CMake](cmake.md) and
[cargo](cargo.md) have their own files, and the flags above can be passed
directly to `clang-cl`.
