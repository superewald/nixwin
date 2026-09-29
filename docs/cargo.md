# cargo and rustc

`rustc.env` configures a rust toolchain for a sysroot. Rust does not read
`clang-cl` flags directly, so this file wires the two halves together: the
`windows-sys` bindings link against the sysroot's import libraries, and the
`cc` build dependency compiles C++ through the LLVM toolchain.

```sh
source "$NIXWIN_SYSROOT/rustc.env"
cargo build --target x86_64-pc-windows-msvc
```

The target must be installed in your toolchain first:

```sh
rustup target add x86_64-pc-windows-msvc
```

## What it sets

```sh
export NIXWIN_SYSROOT="${NIXWIN_SYSROOT:-/home/you/.local/share/nixwin/sysroot}";
export CC=clang-cl
export CXX=clang-cl
export AR=llvm-lib
export NIXWIN_CFLAGS="-fuse-ld=lld-link /winsysroot $NIXWIN_SYSROOT -Xclang -ivfsoverlay -Xclang $NIXWIN_SYSROOT/vfsoverlay.json"
export CFLAGS=$NIXWIN_CFLAGS
export CXXFLAGS=$NIXWIN_CFLAGS
export LIB="$NIXWIN_SYSROOT/VC/Tools/MSVC/14.40.33807/lib/x64;$NIXWIN_SYSROOT/Windows Kits/10/Lib/10.0.26100.0/um/x64;$NIXWIN_SYSROOT/Windows Kits/10/Lib/10.0.26100.0/ucrt/x64"
export RUSTFLAGS="-C linker=lld-link -C link-arg=/vfsoverlay:$NIXWIN_SYSROOT/vfsoverlay.json"
```

| export | why |
|---|---|
| `RUSTFLAGS` `-C linker=lld-link` | `rustc` invokes the linker directly, so it must be told to use `lld-link` rather than the host `cc` |
| `RUSTFLAGS` `-C link-arg=/vfsoverlay:…` | the same case-insensitivity overlay the C side uses, so Windows-style include paths resolve |
| `LIB` | the search path `lld-link` uses for import libraries: MSVC CRT, SDK `um`, SDK `ucrt`, all under the target's MSVC architecture directory. Separators are `;` because `lld-link` is a Windows-style tool on every host. |
| `CC`/`CXX`/`AR` | read by the [`cc` crate](https://crates.io/crates/cc) and by any other build script which shells out to a compiler |
| `CFLAGS`/`CXXFLAGS` | the `/winsysroot` and VFS overlay flags, so C++ compiled from a build script links against the same sysroot |

`LIB` and the version numbers in it are generated per sysroot. If you change
the pinned CRT or SDK version, re-run `nixwin install` so the file is
regenerated.

## A crate that uses it

Nothing in `Cargo.toml` has to mention nixwin. The parts that matter:

```toml
[dependencies]
# links against the windows import libraries through the sysroot
windows-sys = { version = "0.59", features = [
    "Win32_Foundation", "Win32_Graphics_Gdi", "Win32_System_Threading",
    # ...
] }

[build-dependencies]
# compiles and links C++ through CC/CXX/CFLAGS/LIB
cc = "1"
```

`windows-sys` supplies the extern declarations; the libraries themselves are
found through `LIB` inside the sysroot, not vendored. The `cc` crate is the
part that proves `CFLAGS`/`LIB` are right, because it runs `clang-cl` and
`lld-link` for real.

## Architectures

The sysroot must contain the architecture you build for — `x86_64` for
`x86_64-pc-windows-msvc`, `aarch64` for `aarch64-pc-windows-msvc`, and so on:

```sh
nixwin install 17 --archs x86_64,aarch64 --default
rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
```

!!! note "Multi-arch sysroots emit one arch per rustc invocation"

    `rustc.env` is rendered for the sysroot's primary architecture, matching
    what `toolchain.cmake` does. To build for a different architecture, set
    `LIB` yourself for that MSVC directory (`x64`, `arm64`, …) or point
    `NIXWIN_SYSROOT` at a sysroot installed for that architecture.

## A worked example

`examples/rustc` cross-compiles a program which calls into kernel32, user32,
gdi32, shell32, advapi32, ole32, oleaut32 and ws2_32 through `windows-sys`, and
compiles `cpp/extra.cpp` through the `cc` crate.

```sh
cd examples/rustc
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot ./build.sh
```

See [`examples/rustc/README.md`](https://github.com/superewald/nixwin/blob/main/examples/rustc/README.md).
