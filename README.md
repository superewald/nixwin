<p align="center">
    <img src="resources/nixwin_alt.png" width="250rem" alt="nixwin" />
</p>

# nixwin: cross-compile c/c++/rust for windows easily

Create reproducible windows sysroots ready for cross-compiling C/C++/Rust with integration for CMake, LLVM and Cargo within seconds.

- want to cross-compile c/c++/rust targeting windows on unix hosts?
- want faster CI compilation times when compiling for windows?

Nixwin comes to the rescue! 

The windows sysroot files are downloaded from the Visual Studio manifest and composed using [xwin]. Nixwin adds efficiency with a caching layer, consistency through configuration and improves UX by providing integration with common developer tools.

## install nixwin

Download the latest release from [github releases](https://github.com/superewald/nixwin/releases/latest/) and run `nixwin setup`.

```sh
curl -o $HOME/.local/bin/nixwin -L 	https://github.com/superewald/nixwin/releases/latest/download/nixwin-x86_64-musl && \
nixwin setup --wine --cmake # optional, recommended
```


> [!NOTE]
> - `nixwin setup` exports `NIXWIN_SYSROOT`, `NIXWIN_DATA` and `NIXWIN_CACHE` in your shell rc (*see [sysroot/environment variables](./docs/sysroots.md#environment-variables)*)
> - `nixwin setup --cmake` additionally sets `CMAKE_TOOLCHAIN_FILE` in your shell rc
> - `--wine` copies the VCR debug libraries from the default sysroot into your wine prefix (*see [sysroot/wine](./docs/sysroots.md#tool-integration)*)
> - `--cmake` let's cmake detect [lockfiles] and use their [sysroot toolchains](./docs/sysroots.md#tool-integration).

> [!WARNING]
> The nixwin block in your shell rc is rewritten on every `nixwin setup` run to pick up changed paths. Do not edit anything between the `# >>> nixwin >>>` and `# <<< nixwin <<<` markers, your changes are lost on the next run.


## manage sysroots

[Windows sysroots](./docs/sysroots.md) contain necessary files for compiling windows applications (SDK/CRT/VCR headers & libraries). Nixwin provides an efficient interface to install and configure sysroots.

```sh
# install sysroot with latest SDK, CRT and VCR from VS17 manifest
nixwin install 17
# add ATL headers to sysroot
nixwin install 17 --features atl
```

> [!TIP] If you need specific versions use `--manifest`, `--sdk` and `--crt`. See [version pinning](./docs/sysroots.md#version-pinning).

## cross-compile

Beside the files from VS store, nixwin sysroots contain integration files for clang-cl, cmake and rustc for convenience.

> The `NIXWIN_SYSROOT` environment variable is set to the default sysroot and only available after `nixwin setup`.

### clang-cl

```sh
cd examples/llvm
source $NIXWIN_SYSROOT/llvm.env
clang-cl $NIXWIN_LLVM_FLAGS main.cpp -o windows-example.exe
```

### cmake

```sh
cd examples/cmake
source $NIXWIN_SYSROOT/cmake.env # not necessary with `nixwin setup --cmake`
cmake -S . -B build
cmake --build build
```

### rustc

```sh
cd examples/rustc
source $NIXWIN_SYSROOT/rustc.env
cargo build -C examples/rustc --target x86_64-pc-windows-msvc
```

## Lockfiles

A lockfile is a `.nixwin.json` file which pins a sysroot to a fully resolved set of components.. Commit a lockfile to version control to make installs reproducible across machines and CI runs.

```sh
nixwin install 17 --lock
```

```sh
git clone https://github.com/superewald/nixwin
cd nixwin/examples/
nixwin install
nixwin install --lock --features atl
git commit -m "chore: added ATL to nixwin sysroot"
```

> See [docs/lockfiles.md](./docs/lockfiles.md)

## Docker/Podman

Nixwin provides docker images with bundled sysroots for all supported VS manifest versions (16/17/18).

```sh
docker run --rm -it \
	-v $PWD/examples/llvm:/app \
	-v $PWD/nixwin-data:/share/nixwin \
	superewald/nixwin:17 
	
```

> See [docker](./docker/README.md).

## CI

Nixwin is designed to run efficiently in CI and allows to skip heavy windows docker images. For use in CI, it is recommended to explicitly set `--cache-dir` (`$NIXWIN_CACHE`) and `--data-dir` (`$NIXWIN_DATA`) to create cacheable and reproducible windows sysroots. 

You can install nixwin manually into your CI containers but it is recommended to use the [nixwin container images](#dockerpodman).

- [GitHub CI](./examples/.github.ci.yml)
- [GitLab CI](./examples/.gitlab.ci.yml)

[xwin]: https://github.com/jake-shadle/xwin
[lockfiles]: ./docs/lockfiles.md