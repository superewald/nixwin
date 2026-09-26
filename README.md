<p align="center">
    <img src="resources/nixwin_alt.png" width="250rem" alt="nixwin" />
</p>

Create reproducible windows sysroots ready for cross-compiling C/C++/Rust with integration for CMake, LLVM and Cargo within seconds.

- want to cross-compile c/c++/rust targeting windows on unix hosts?
- want faster CI compilation times when compiling for windows?

Nixwin comes to the rescue! 

The windows sysroot files are downloaded from the Visual Studio manifest and composed using [xwin]. Nixwin adds efficiency with a caching layer, consistency through configuration and improves UX by providing integration with common developer tools.

# Quickstart

#### 1. Install nixwin

```sh
curl -o $HOME/.local/bin/nixwin -L \
    https://github.com/superewald/nixwin/releases/latest/download/nixwin-x86_64-musl
```

#### 2. Setup [tool integration](#tool-integration) *(optional, recommended)*

```sh
nixwin setup --wine --cmake
```

> [!NOTE]
> 
> - `nixwin setup` adds `NIXWIN_` env variables to `~/.bashrc`
> - `--wine` adds VCR debug libraries to wine prefixes *(required to run debug builds in wine)*
> - `--cmake` sets `CMAKE_TOOLCHAIN_FILE` in `~/.bashrc` to a wrapper script that detects [toolchain lockfiles](#lockfiles) (`.nixwin.json`)

#### 3. Install sysroot 

```sh
nixwin install 17
```

> [!NOTE]
> Pulls the latest SDK/CRT files of the VS 17 manifest and compose them into a llvm-compatible [windows sysroot](#windows-sysroots) layout at `$HOME/.local/share/nixwin/sysroots/17`. *See [install command](#install) docs for details.*

#### 4. Try the [examples](examples/README.md) *(optional)*

```sh
nixwin install 17 --default
examples/llvm/build.sh
examples/cmake/build.sh
examples/rustc/build.sh
```

> [!NOTE]
> Each example cross-compiles the same small program against the sysroot, exercising the emitted [tool integration files](#tool-integration) (`llvm.env`, `toolchain.cmake`, `rustc.env`), and runs it under wine when available.

## Windows Sysroots

A windows sysroot contains necessary sources and libraries to cross-compile c/c++/rust targeting windows on unix hosts. Additionally, nixwin adds convenience scripts and configurations for seamless integration into common developer tools.

- nixwin supports coexisting sysroots for different target environments with efficient caching
- sysroots reside in `$NIXWIN_DATA/sysroots` and symlink against the shared cache `$NIXWIN_CACHE`. 
- every sysroot has a tag assigned for identification which resolves to `$NIXWIN_DATA/sysroots/$TAG`. 

### managing sysroots

```sh
# install a windows sysroot with latest CRT/SDK from VS17 manifest
nixwin install --tag 17 --manifest 17
# add debug feature and x86 architecture to the sysroot
nixwin install --tag 17 --features debug --archs x86
# set specific sysroot as user default
nixwin config default.tag 17
# list installed sysroots
nixwin ls
# print details about a specific sysroot
nixwin inspect 17
# remove a specific sysroot
nixwin rm 17
```

### what's included?

The following windows components are provided within the sysroot. Component versions default to the latest versions in the selected vs manifest.
By default, components only include resources necessary for release builds. Debug libraries/headers can be installed using the `debug` feature.

| component | feature | description | notes |
|---|---|---|---|
| **SDK** | - | Windows SDK headers & libraries | *required, contains resources to build for release* |
| **CRT** | - | Windows CRT headers & libraries | *required, contains resources to build for release* |
| **SDK/CRT** | `debug` | Include debug libraries and symbols for SDK/CRT | *needed to compile debug builds* |
| **VCR** | `debug` | Include VCR debug runtime libraries | *needed to run debug builds in wine* |
| **ATL** | `atl` | Include [ATL templates](https://learn.microsoft.com/en-us/cpp/atl/active-template-library-atl-concepts) |

The `debug` feature is enabled by default unless a CI environment is detected.

### tool integration

Nixwin adds convenience configurations to integrate the windows sysroot with common developer tools.

| file | tools | details |
|---|---|---|
| `vfsoverlay.json` | clang/lld-link | A vfsoverlay file for the windows sysroot which avoids issues with filesystem case-sensitivity. |
| `llvm.env` | clang/lld-link | Configures clang/lld-link using `CFLAGS`/`CXXFLAGS`. |
| `toolchain.cmake` | cmake | Provides a cross-compilation toolchain for CMake using llvm. |
| `cmake.env` | cmake | Configures cmake using `toolchain.cmake` and `vfsoverlay.json`. |
| `rustc.env` | rustc/cargo | Configures rustc using `RUSTFLAGS`/`CFLAGS`/`CXXFLAGS` |

### lockfiles

A lockfile is a `.nixwin.json` file which pins a sysroot to a fully resolved set of components: its `tag`, `manifest`, `channel`, `archs`, `variants`, `features`, `sdk`, `crt` and `vcr`. Commit a lockfile to version control to make installs reproducible across machines and CI runs.

Lockfiles are written to two places:

- `$CWD/.nixwin.json` — the project lockfile, written on demand with `nixwin install --lock`
- `$NIXWIN_DATA/sysroots/$TAG/.nixwin.json` — the configuration of an installed sysroot, emitted on every install

This is separate from the machine configuration (`$NIXWIN_DATA/config.json`, see [config](#config)), which holds per-machine install defaults and integration flags rather than pinned versions.

<details><summary><b>creating a lockfile</b></summary>

```sh
# create .nixwin.json for the VS 16 sysroot in the current directory
nixwin install 16 --lock
# create .nixwin.json in another project directory
nixwin install 16 --lock /path/to/project
# write to an explicit file instead
nixwin install 16 --lock path/to/custom.lock.json
```

`--lock` takes an optional path. A directory receives the default `.nixwin.json` name, any other path is written verbatim.

</details>

<details><summary><b>how lockfiles are used</b></summary>

| consumer | behavior |
|---|---|
| `nixwin install` | reads `$CWD/.nixwin.json` unless `--config <PATH>` points at another lockfile |
| `--config <PATH>` | global flag, installs from the lockfile at an explicit path |
| `nixwin setup --cmake` | the generated `CMAKE_TOOLCHAIN_FILE` wrapper scans `${CMAKE_CURRENT_SOURCE_DIR}` and its own directory for `.nixwin.json`, and uses the `tag` therein to include `$NIXWIN_DATA/sysroots/$TAG/toolchain.cmake`. Falls back to `$NIXWIN_SYSROOT` and then `$NIXWIN_DATA/sysroot` when no lockfile is found. |
| CI | hash the lockfile into your cache key so the cache is invalidated when component versions change |

When resolving an install, lockfile values are combined with flags as follows:

- scalars (`tag`, `manifest`, `channel`, `sdk`, `crt`): flags > lockfile > installed sysroot configuration > machine defaults > builtin defaults
- lists (`archs`, `variants`, `features`): the union of the installed sysroot configuration, the lockfile and the flags. Machine defaults are only applied when that union is empty.

</details>


<details><summary><b>environment variables</b></summary>

| variable | description | default |
|---|---|---|
| `NIXWIN_SYSROOT` | path to default sysroot | `$NIXWIN_DATA/sysroot` |
| `NIXWIN_SYSROOTS` | path to sysroot directories | `$NIXWIN_DATA/sysroots` |
| `NIXWIN_CACHE` | path to cache directory | `$HOME/.cache/nixwin` |
| `NIXWIN_CACHE_MSVC` | path to msvc cache dir | `$NIXWIN_CACHE/VC/Tools/MSVC` |
| `NIXWIN_CACHE_SDK` | path to sdk cache dir | `$NIXWIN_CACHE/Windows Kits/10` |
| `NIXWIN_CACHE_VCR` | path to vcr cache dir | `$NIXWIN_CACHE/VCR` |
| `NIXWIN_DATA` | path to data directory | `$HOME/.local/share/nixwin` |

</details>

<details><summary><b>cache directory layout</b></summary>

```sh
$NIXWIN_CACHE/
    VC/Tools/MSVC/${CRT_VERSIONS...}/
        include/
        lib/${ARCHS...}/
    VCR/${VCR_VERSIONS...}/bin/${ARCHS...}/
    Windows Kits/{10/11}/
        Include/${SDK_VERSIONS...}/
        Lib/${SDK_VERSIONS...}/{ucrt,um}/${ARCHS...}

NIXWIN_CACHE_MSVC=$NIXWIN_CACHE/VC/Tools/MSVC/
NIXWIN_CACHE_VCR=$NIXWIN_CACHE/VCR
NIXWIN_CACHE_SDK=$NIXWIN_CACHE/Windows Kits/{10/11}/
```

</details>

<details><summary><b>data directory layout</b></summary>

```sh
$NIXWIN_DATA/
    sysroots/$TAG/
        VC/Tools/MSVC/$CRT_VERSION/
            include/            # ln -s $NIXWIN_CACHE_MSVC/$CRT_VERSION/include
            lib/${ARCHS...}/    # ln -s $NIXWIN_CACHE_MSVC/$CRT_VERSION/lib/${ARCHS...}
        VCR/$VCR_VERSION/bin/${ARCHS...}/ # ln -s $NIXWIN_CACHE_VCR/$VCR_VERSION/bin/${ARCHS...}
        Windows Kits/{10/11}/
            Include/$SDK_VERSION/ # ln -s $NIXWIN_CACHE_SDK/Include/$SDK_VERSION
            Lib/$SDK_VERSION/ # ln -s $NIXWIN_CACHE_SDK/Lib/$SDK_VERSION
        vfsoverlay.json
        toolchain.cmake
        cmake.env
        rustc.env
        llvm.env
        .nixwin.json
    sysroot     # ln -s sysroots/$(nixwin config default.tag)
    config.json 

```

</details>

## CI

Nixwin is designed to run efficiently in CI and allows to skip heavy windows docker images. For use in CI, it is recommended to explicitly set `--cache-dir` (`$NIXWIN_CACHE`) and `--data-dir` (`$NIXWIN_DATA`) to create cacheable and reproducible windows sysroots. 

You can install nixwin manually into your CI containers but it is recommended to use the [nixwin container images](#dockerpodman).

<details><summary><b>GitHub</b></summary>

```yaml
name: ci
on: [push, pull_request]
jobs:
  cross:
    runs-on: ubuntu-latest
    container: { image: nixwin:17 }
    env:
      NIXWIN_CACHE: /tmp/nixwin-cache
      NIXWIN_DATA: /tmp/nixwin-data
    steps:
      - uses: actions/checkout@v4
      - uses: actions/cache@v4
        with:
          path: [ "${{ env.NIXWIN_CACHE }}", "${{ env.NIXWIN_DATA }}" ]
          key: nixwin-17-${{ hashFiles('.nixwin.json') }}
          restore-keys: [ nixwin-17- ]
      - run: nixwin install 17 --cache-dir "$NIXWIN_CACHE" --data-dir "$NIXWIN_DATA" --features debug
      - run: |
          export CMAKE_TOOLCHAIN_FILE=$NIXWIN_SYSROOT/toolchain.cmake
          export CMAKE_CLANG_VFS_OVERLAY=$NIXWIN_SYSROOT/vfsoverlay.json
          cmake -B build -DTARGET_PLATFORM Win32 && cmake --build build
          source $NIXWIN_SYSROOT/rustc.env
          cargo build --target x86_64-pc-windows-msvc
```

</details>

<details><summary><b>GitLab</b></summary>

```yaml
variables:
  NIXWIN_CACHE: nixwin-cache
  NIXWIN_DATA: nixwin-data
build:
  image: nixwin:17
  cache:
    key: { files: [.nixwin.json] }
    paths: [nixwin-cache/, nixwin-data/]
  before_script:
    - nixwin install 17 --cache-dir "$NIXWIN_CACHE" --data-dir "$NIXWIN_DATA" --features debug
  script:
    - export CMAKE_TOOLCHAIN_FILE=$NIXWIN_SYSROOT/toolchain.cmake
    - export CMAKE_CLANG_VFS_OVERLAY=$NIXWIN_SYSROOT/vfsoverlay.json
    - cmake -B build -DTARGET_PLATFORM Win32 && cmake --build build
    - source $NIXWIN_SYSROOT/rustc.env && cargo build --target x86_64-pc-windows-msvc
```

</details>

## Docker/Podman

Nixwin provides docker images with bundled sysroots for all supported vs manifest versions.

**base images**

Base images contain sysroots (CRT/SDK) for x86_64 platform and common compilation tools (llvm, cmake, make, ...). Nixwin provides images based on ubuntu and alpine: `nixwin:$VS_VERSION` and `nixwin:$VS_VERSION-alpine`.

- `nixwin:16[-alpine]`
- `nixwin:17[-alpine]`
- `nixwin:18[-alpine]`

**flavors**

Additionally, each base image has the following feature extensions:

- `nixwin:$VS_VERSION-debug`: adds debug libraries
- `nixwin:$VS_VERSION-wine`: adds debug libraries and wine
- `nixwin:$VS_VERSION-aarch`: adds aarch/aarch64 platform
- `nixwin:$VS_VERSION-aarch-debug`: adds aarch/aarch64 platform and debug libraries
- `nixwin:$VS_VERSION-aarch-wine`: adds aarch/aarch64 platform, debug libraries and wine

**seamless docker integration**

The nixwin image can be used locally for neat encapsulation:

```sh
docker pull nixwin:17-wine
alias nixwin="docker run -it --rm -v \"$PWD:/app\" -w /app nixwin:17-wine nixwin"
nixwin ls # VS 17
nixwin install
```

# Commands

## setup

**Synopsis**: `nixwin setup [OPTIONS]`

Initialize nixwin on the machine, create configuration and tool integration.

```sh
# setup nixwin with default settings
nixwin setup 
nixwin setup --cmake --wine
```

**Options**
| option |  description |
|---|---|
| `--cmake` | Set `CMAKE_TOOLCHAIN_FILE` to a wrapper which auto-selects the sysroot toolchain when `.nixwin.json` is present. | false | | |
| `--wine` | Add the VCR debug libraries to user wine prefixes.

## install

**Synopsis**: `nixwin install [MANIFEST_VERSION] [OPTIONS]`

Install pre-configured [windows sysroots](#windows-sysroots) from VS package store.

The `MANIFEST_VERSION` argument is a shortuct for `nixwin install --tag $MANIFEST_VERSION --manifest $MANIFEST_VERSION`.

**Examples**

```sh
# install sysroots from .nixwin.json
nixwin install
# install latest sdk/crt versions from VS 16
nixwin install 16 # shortcut for `nixwin install -t 16 --manifest 16`
# add a feature/architecture to the VS 16 windows sysroot
nixwin install 16 -a x86 -f debug
# create .nixwin.json for VS 16 sysroot in current directory
nixwin install 16 --lock
# install latest versions of sdk/crt, tag the sysroot as `25H2` and create lockfile
nixwin install --tag 25H2 \
    --manifest 18 \
    --sdk "10.0.28000.0" \
    --crt "14.40.33807" \
    --features debug \
    --archs x86_64,aarch64 \
    --variants desktop,onecore,spectre \
    --lock /path/to/project
# create .nixwin.json for 25H2 sysroot in the current directory
nixwin install -t 25H2 --lock
```

**Options**

| option |  description | default | values | required |
|---|---|---|---|---|
| `-t`, `--tag` | Name of the windows sysroot | manifest version *(see `--manifest`)* | name must use valid path characters | 🗹 |
| `-a`, `--archs` | Target architectures to include. | Host system architecture | `[x86,x86_64,aarch,aarch64]` | ☐ |
| `-f`, `--features` | Features to include. | `debug` unless CI | `[debug,atl,wdk]` | ☐ |
| `--variants` | SDK/CRT variants to include. | `desktop` | `[desktop,onecore,store,spectre]` | ☐ |
| `--manifest` | Visual Studio manifest version to use. | `MANIFEST_VERSION` or `17` | `[16,17,18]` | ☐ |
| `--channel` | Visual Studio manifest channel to use. | `release` | eg `release`, `pre` | ☐ |
| `--sdk` | Use a specific WindowsSDK version. | - | eg `10.0.28000.0` | ☐ |
| `--crt` | Use a specific CRT version (this also determines the VCR version). | - | eg `14.40.33807` | ☐ |
| `-d`, `--default` | Set this windows sysroot as user default (*see [default](#default)*) | - | | ☐ |
| `--lock` | Path to the lockfile to write. If not specified no lockfile is generated. | `$CWD/.nixwin.json` | valid path | ☐ |

## config

**Synopsis**: `nixwin config [KEY] [VALUE] [--unset]`

Get or set values from the nixwin configuration (`$NIXWIN_DATA/config.json`). Without arguments, prints the whole configuration. With only `KEY`, prints the current value. With `KEY` and `VALUE`, sets it. `--unset KEY` removes it.

```sh
# print the whole configuration
nixwin config
# get/set the default sysroot (also updates the `$NIXWIN_SYSROOT` link)
nixwin config default.tag
nixwin config default.tag 16
# configure machine-wide install defaults (used when a flag is not given)
nixwin config default.arches x86,x86_64
nixwin config default.variants desktop
nixwin config default.features debug,atl
nixwin config default.manifest 17
nixwin config default.channel release
# override an emitted file's template source
nixwin config tpl.toolchain /path/to/custom/toolchain.tpl
```

| key | description | values |
|---|---|---|
| `default.tag` | default windows sysroot | installed sysroot tag |
| `default.arches` | default target architectures | `[x86,x86_64,aarch,aarch64]` |
| `default.variants` | default SDK/CRT variants | `[desktop,onecore,store,spectre]` |
| `default.features` | default features | `[debug,atl]` |
| `default.manifest` | default VS manifest version | `[16,17,18]` |
| `default.channel` | default VS manifest channel | eg `release` |
| `cmake` / `wine` | integration flags (set by `setup`) | `true`/`false` |
| `tpl.<name>` | template override for an emitted file | `toolchain`, `cmake`, `rustc`, `llvm`, `cmake-wrapper` |

## ls

**Synopsis**: `nixwin ls`

List the installed windows sysroots.

## rm

**Synopsis**: `nixwin rm [TAG]`

Removes the windows sysroot from disk.

## inspect

**Synopsis**: `nixwin inspect [TAG]`

Print details about a windows sysroot.

```sh
$ nixwin inspect 16
tag: 16
sdk: "10.0.19041.0"
crt: "14.29.30133"
vcr: "14.29.30133"
variants: [desktop]
features: [debug,atl]
```


[xwin]: https://github.com/jake-shadle/xwin