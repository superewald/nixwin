# Examples

Each example cross-compiles a small Windows program — touching the most common
base classes (kernel32, user32, gdi32, shell32, advapi32, ole32, oleaut32,
ws2_32) — against the nixwin sysroot using the integration files emitted by
`nixwin install`, and runs it with wine when available.

| directory   | toolchain                                                  |
|-------------|------------------------------------------------------------|
| `llvm/`     | clang-cl + lld-link (direct invocation, including the UCRT and vc redist libraries automatically) |
| `cmake/`    | CMake with the generated `toolchain.cmake`                 |
| `rustc/`    | cargo with the `rustc.env` environment (CC/CXX/AR, CFLAGS, LIB, RUSTFLAGS) |

Prerequisites:

```bash
nixwin setup
nixwin install 17 --default
```

The cmake and llvm examples also need a clang/lld toolchain and CMake; the
rustc example needs cargo plus the `x86_64-pc-windows-msvc` target
(`rustup target add x86_64-pc-windows-msvc`), and it cross-compiles a `cc`
dependency (`cpp/extra.cpp`) through the CC/CXX/CFLAGS environment.

Build and run an example:

```bash
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/llvm/build.sh
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/cmake/build.sh
NIXWIN_SYSROOT=$HOME/.local/share/nixwin/sysroot examples/rustc/build.sh
```

When the sysroot is the default `$HOME/.local/share/nixwin/sysroot`, the
`NIXWIN_SYSROOT` override is optional.

Each program prints a few lines describing the environment, e.g.:

```
pid: 32
module: Z:\...\windows-example.exe
screen: 1920x1080
bpp: 32
argc: 1
windows: Windows 10 Pro
socket: 120
```

If `wine` is not installed the executables are linked into `build/` /
`target/` and only the build happens.