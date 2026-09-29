# rustc example

Cross-compiles a Rust program that links the same set of Windows libraries as
the other two examples, but through `windows-sys` and a build script instead of
a `CMakeLists.txt` or a bare compiler invocation. It is the only example that
compiles *two* languages: `src/main.rs` is Rust, and `cpp/extra.cpp` is C++
built by the [`cc`](https://crates.io/crates/cc) crate.

Prerequisites beyond the [shared ones](../README.md#prerequisites):

- `cargo`/`rustc`
- the Windows target installed:
  ```sh
  rustup target add x86_64-pc-windows-msvc
  ```

The sysroot must contain `x86_64` as well, which the sysroot installed by the
[shared prerequisites](../README.md#prerequisites) does.

## Build

```sh
./build.sh
```

or, to see the individual steps:

```sh
source $NIXWIN_SYSROOT/rustc.env
cargo build --target x86_64-pc-windows-msvc
```

The binary lands in
`target/x86_64-pc-windows-msvc/debug/rustc-example.exe`.

## The emitted environment

`rustc.env` is rendered per sysroot, so its `LIB` entry contains that sysroot's
actual CRT and SDK versions:

```sh
export NIXWIN_SYSROOT="${NIXWIN_SYSROOT:-...}";
export CC=clang-cl
export CXX=clang-cl
export AR=llvm-lib
export NIXWIN_CFLAGS="-fuse-ld=lld-link /winsysroot $NIXWIN_SYSROOT -Xclang -ivfsoverlay -Xclang $NIXWIN_SYSROOT/vfsoverlay.json"
export CFLAGS=$NIXWIN_CFLAGS
export CXXFLAGS=$NIXWIN_CFLAGS
export LIB=".../VC/Tools/MSVC/<crt>/lib/x64;.../Windows Kits/10/Lib/<sdk>/um/x64;.../Windows Kits/10/Lib/<sdk>/ucrt/x64"
export RUSTFLAGS="-C linker=lld-link -C link-arg=/vfsoverlay:$NIXWIN_SYSROOT/vfsoverlay.json"
```

Two things differ from the other integrations:

- **`RUSTFLAGS` names the linker explicitly.** `rustc` invokes the linker
  directly rather than through a compiler driver, so it needs
  `-C linker=lld-link`; `CFLAGS` alone would not reach it. The
  `-C link-arg=/vfsoverlay:…` is the same case-insensitivity overlay the C side
  gets.
- **`LIB` is a pre-computed search path**, using `;` separators because
  `lld-link` is a Windows-style tool on every host platform. It lists the MSVC
  CRT, the SDK `um` and the SDK `ucrt` for the sysroot's primary architecture.
  This is where `windows-sys`'s import libraries are found — nothing is vendored
  into the crate.

## The cross-compiled dependency

`build.rs` uses the `cc` crate to compile `cpp/extra.cpp` into a static library
and link it. That is the part of this example which exercises `CC`, `CXX`,
`CFLAGS` and `LIB`: a build script shelling out to `clang-cl` and `lld-link`
against the same sysroot as cargo.

```rust
// build.rs
fn main() {
    cc::Build::new().file("cpp/extra.cpp").compile("nixwin_example");
}
```

`src/main.rs` then calls the C++ function through an `extern "C"` declaration,
and prints its result as the last line:

```
cc pid: 32
```

If that line matches the `pid:` line, the C++ half linked and ran correctly,
which means the environment from `rustc.env` reached the build script as well as
cargo.

## Notes

- Nothing in `Cargo.toml` mentions nixwin. `windows-sys` supplies the extern
  declarations and the `cc` crate does the C++ build; the sysroot supplies
  everything else through the environment.
- For a different architecture, install it in the sysroot
  (`nixwin install 17 --archs aarch64 --default`) and add the matching rustup
  target. `rustc.env` is rendered for the sysroot's primary architecture.
- See the [cargo guide](../../docs/cargo.md) for details, including how to
  override the environment per project with `RUSTFLAGS` and `LIB` from
  `.cargo/config.toml`.
