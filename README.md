<p align="center">
    <img src="resources/nixwin_alt.png" width="250rem" alt="nixwin" />
</p>

# nixwin: cross-compile c/c++/rust for windows easily

Create reproducible windows sysroots ready for cross-compiling C/C++/Rust with integration for CMake, LLVM and Cargo within seconds.

- want to cross-compile c/c++/rust targeting windows on unix hosts?
- want faster CI compilation times when compiling for windows?

Nixwin comes to the rescue!

The windows sysroot files are downloaded from the Visual Studio manifest and composed using [xwin]. Nixwin adds efficiency with a caching layer, consistency through configuration and improves UX by providing integration with common developer tools.

## quickstart

```sh
# 1. install the binary into ~/.local/bin
curl -fsSL https://raw.githubusercontent.com/superewald/nixwin/main/install.sh | bash

# 2. create the data/cache directories and export them to your shell
nixwin setup

# 3. install a sysroot and make it the default
nixwin install 17 --default

# 4. cross-compile
./examples/llvm/build.sh
```

Step 4 produces `windows-example.exe` and runs it under wine if wine is
installed. `nixwin setup` writes to your shell rc, so **open a new shell** (or
`source ~/.zshrc`) before step 3 if you ran step 2 in an existing one.

Prefer a specific release over the latest, or a different install directory? See
[`install.sh --help`](./install.sh), or the
[install script docs](https://superewald.github.io/nixwin/#quick-install).

## what's inside a sysroot

A [sysroot](./docs/sysroots.md) holds the Windows SDK and MSVC CRT headers,
import libraries and static libraries, laid out the way `clang-cl` expects, plus
a `vfsoverlay.json` that makes case-insensitive Windows includes resolve on a
case-sensitive host. By default it contains only what a release build needs; add
`--features debug` for debug libraries and the VCR debug runtime.

> The `debug` feature is on by default unless a CI environment is detected.

## manage sysroots

Sysroots are tagged and share one component cache, so keeping several around
costs disk only for what actually differs.

```sh
nixwin ls                          # list installed sysroots
nixwin inspect 17                  # versions, arches, features of one sysroot
nixwin config default.tag 17       # switch which sysroot is the default
nixwin install 17 --features atl   # add a feature to an existing sysroot
nixwin rm 17                       # remove a whole sysroot
nixwin rm 17 -a aarch64            # or just one architecture from it
```

> Need exact versions? Use `--manifest`, `--sdk` and `--crt`. See
> [version pinning](./docs/sysroots.md#version-pinning).

## tool integration

Alongside the component files, each sysroot emits integration files so a build
is configured by sourcing one file instead of assembling flags by hand.

| file | tools | guide |
|---|---|---|
| `llvm.env` | clang-cl + lld-link | [clang-cl and lld-link](./docs/llvm.md) |
| `toolchain.cmake`, `cmake.env` | cmake | [CMake](./docs/cmake.md) |
| `rustc.env` | rustc/cargo | [cargo and rustc](./docs/cargo.md) |
| `WINEPATH` (via `nixwin setup --wine`) | wine | [wine](./docs/wine.md) |

```sh
source $NIXWIN_SYSROOT/llvm.env
clang-cl $NIXWIN_LLVM_FLAGS main.cpp -o windows-example.exe
```

`nixwin setup --cmake` additionally writes a *wrapper* toolchain that picks the
sysroot a project's lockfile names, so `cmake -B build` needs no flags at all.
See [the lockfile-aware wrapper](./docs/cmake.md#the-lockfile-aware-wrapper).

> `NIXWIN_SYSROOT` points at the default sysroot and is only set after
> `nixwin setup`.

### environment quicksheet

| variable | what it does |
|---|---|
| `NIXWIN_SYSROOT` | the default sysroot; the tools above are generated for it |
| `NIXWIN_DATA` | installed sysroots and the machine config |
| `NIXWIN_CACHE` | shared component cache, reused by every sysroot |

> The nixwin block in your shell rc is rewritten on every `nixwin setup` run, so
> a changed `--data-dir` or `--cache-dir` is picked up. Do not edit anything
> between the `# >>> nixwin >>>` and `# <<< nixwin <<<` markers, your changes are
> lost on the next run. The full list, including the ones `nixwin` writes
> (`CMAKE_TOOLCHAIN_FILE`, `WINEPATH`), is in
> [configuration and environment](./docs/configuration.md).

## lockfiles

A lockfile is a `.nixwin.json` which pins a sysroot to exact component versions.
Commit it and every machine — including CI — installs the same bytes.

```sh
nixwin install 17 --lock   # write .nixwin.json into the current directory
nixwin install             # in another clone: install exactly what it pins
```

> List-valued components (architectures, variants, features) are the *union* of
> the lockfile, the flags and the installed sysroot, so a lockfile which still
> lists a component you removed with `nixwin rm` brings it back. Update the
> lockfile in the same change as the removal. See
> [docs/lockfiles.md](./docs/lockfiles.md).

## CI

Nixwin is designed to run efficiently in CI and lets you skip heavy Windows
container images. Set `--cache-dir` (`$NIXWIN_CACHE`) and `--data-dir`
(`$NIXWIN_DATA`) to a cacheable location so successive jobs reuse the downloaded
components, and hash `.nixwin.json` into the cache key so it is invalidated when
the pinned versions change.

- [GitHub CI](./examples/.github.ci.yml)
- [GitLab CI](./examples/.gitlab.ci.yml)

## Docker/Podman

Prebuilt images bundle a sysroot with the common compilation tools, one per
supported VS manifest version (16/17/18) with `-alpine`, `-debug`, `-wine` and
`-aarch` flavors. The images are built outside this repository, so there is no
Dockerfile here to build from.

```sh
docker run --rm -it \
	-v $PWD/examples/llvm:/app \
	-v $PWD/nixwin-data:/share/nixwin \
	nixwin:17
```

> See [container images](./docs/docker.md) for the full tag list and a CI
> example.

## documentation

The full documentation is published at **[nixwin documentation site](https://superewald.github.io/nixwin/)**
and built from [`docs/`](./docs):

- [Sysroots](./docs/sysroots.md) — contents, layout on disk, managing several
- [Command reference](./docs/cli.md) — every command and flag
- [Configuration and environment](./docs/configuration.md) — variables and `nixwin config` keys
- [Lockfiles](./docs/lockfiles.md) — reproducible installs
- [Tool integration](./docs/llvm.md) — [clang-cl](./docs/llvm.md), [CMake](./docs/cmake.md), [cargo](./docs/cargo.md), [wine](./docs/wine.md)
- [Container images](./docs/docker.md)
- [Examples](./examples/README.md) — one cross-compiled program per toolchain

[xwin]: https://github.com/jake-shadle/xwin
