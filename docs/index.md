# nixwin

`nixwin` builds a **windows sysroot** on a unix host, so you can cross-compile
C, C++ and Rust for Windows from Linux or macOS. It pulls the MSVC CRT, the
Windows SDK and the VCR redistributables from Microsoft's Visual Studio package
store, arranges them into a compiler-ready tree, and wires the result into
`clang-cl`, `lld-link`, CMake, cargo and wine.

```sh
nixwin install 17 --default
nixwin setup
clang-cl main.cpp -o app.exe
```

## Quick install

```sh
curl -fsSL https://raw.githubusercontent.com/superewald/nixwin/main/install.sh | bash
```

That is the same script the site serves at
[`/install/`](https://superewald.github.io/nixwin/install/). It detects your platform,
resolves the latest published release, and installs the matching binary. If you
would rather pick a release yourself, take the archive from
[`/download/latest/`](https://superewald.github.io/nixwin/download/latest/).

### What the script needs

| requirement | notes |
|---|---|
| `bash` | the script is `#!/usr/bin/env bash` and uses `set -euo pipefail` |
| `curl` or `wget` | either one is used to talk to the GitHub API and download the release |
| standard POSIX tools | `uname`, `sed`, `awk`, `basename`, `mktemp` |
| a supported host | `x86_64`/`amd64`, `aarch64`/`arm64`, `i386`–`i686` or `armv6l`/`armv7l`, on `Linux` or `Darwin` |
| a writable home directory | the install directory must resolve to somewhere inside `$HOME` |

### What the script will not do

It never escalates privileges. It runs no `sudo`, installs no packages, and
writes only inside your home directory: the install target plus a temporary
directory that is removed on exit. It does not edit `~/.zshrc` or `~/.bashrc`;
if the install directory is not already on `PATH` it prints the `export` line
you would need.

### Options

Every option is optional; running the script with none installs the latest
release into `$HOME/.local/bin/nixwin`.

| option | description |
|---|---|
| `--bin-dir <dir>` | install into `<dir>` instead of `$HOME/.local/bin`. Must be inside your home directory. |
| `--version <tag>` | install a specific release tag instead of the latest one. |
| `--dry-run` | print the release, asset, platform and target, then stop without downloading or writing. |
| `-h`, `--help` | print the usage summary. |

`NIXWIN_INSTALL_BIN_DIR` is the environment equivalent of `--bin-dir`.

```sh
# see what would happen, then install it
curl -fsSL https://raw.githubusercontent.com/superewald/nixwin/main/install.sh | bash -s -- --dry-run

# install a pinned release into a project-local directory
curl -fsSL https://raw.githubusercontent.com/superewald/nixwin/main/install.sh | bash -s -- --version v0.2.0 --bin-dir ./bin
```

!!! note "No release published yet?"

    If the repository has no published release, the script says so and stops
    with a link to the source. Build from the repository instead:

    ```sh
    git clone https://github.com/superewald/nixwin
    cd nixwin && cargo build --release
    ```

## First steps

```sh
# 1. create the data and cache directories and export them to your shell
nixwin setup

# 2. install a sysroot and make it the default
nixwin install 17 --default

# 3. cross-compile
./examples/llvm/build.sh
```

`nixwin setup` writes `NIXWIN_SYSROOT`, `NIXWIN_DATA` and `NIXWIN_CACHE` into
your shell rc, so every later command and every build tool sees the same paths.

## What you get

- **A real MSVC sysroot.** The Windows SDK and the MSVC CRT headers, import
  libraries and static libraries, laid out in the directory structure `clang-cl`
  expects, with a `vfsoverlay.json` that makes the case-insensitive Windows
  headers work on a case-sensitive host filesystem.
- **More than one sysroot at a time.** Sysroots are tagged and share a single
  component cache, so keeping a VS 17 and a VS 18 sysroot side by side costs
  disk only for what actually differs. `nixwin config default.tag` switches
  which one is the default.
- **Trimmable installs.** Ask for the architectures, SDK/CRT variants and
  features you need, and remove them again later with `nixwin rm` without
  re-downloading anything.
- **Generated tool integrations.** `llvm.env`, `cmake.env`, `rustc.env`,
  `toolchain.cmake` and a lockfile-aware CMake toolchain wrapper, so a build is
  configured by sourcing one file instead of by assembling flags by hand.
- **Reproducible installs.** `.nixwin.json` pins a sysroot to exact component
  versions; commit it and a CI run installs the same bytes as your laptop.
- **A container image per manifest version**, for builds that should not touch
  the host at all.

## Next steps

- [Sysroots](sysroots.md) — what is inside one, how they are laid out on disk,
  and how to manage several.
- [Command reference](cli.md) — every command and flag.
- [Configuration and environment](configuration.md) — the variables nixwin reads
  and the keys `nixwin config` accepts.
- [Lockfiles](lockfiles.md) — pinning components for reproducible installs.
- Tool integration — [clang-cl and lld-link](llvm.md), [CMake](cmake.md),
  [cargo](cargo.md) and [wine](wine.md).
- [Container images](docker.md) — prebuilt images for CI.
