# Windows Sysroots

A windows sysroot contains necessary sources and libraries to cross-compile c/c++/rust targeting windows on unix hosts. Additionally, nixwin adds convenience scripts and configurations for seamless integration into common developer tools.

- nixwin supports coexisting sysroots for different target environments with efficient caching
- sysroots reside in `$NIXWIN_DATA/sysroots` and symlink against the shared cache `$NIXWIN_CACHE`.
- every sysroot has a tag assigned for identification which resolves to `$NIXWIN_DATA/sysroots/$TAG`.

## managing sysroots

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

## version pinning

#### install exact versions
 Use `--manifest`, `--sdk` and `--crt` to pin exact versions of the components. For example, to install the latest SDK and CRT targeting Windows 11 25H2:
 ```sh
 nixwin install --tag 25H2 --manifest 18 \
 	--sdk 10.0.26100.0 \
	--crt 14.40.33807
 ```

## what's included?

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

## tool integration

Nixwin adds convenience configurations to integrate the windows sysroot with common developer tools.

| file | tools | details | guide |
|---|---|---|---|
| `vfsoverlay.json` | clang/lld-link | A vfsoverlay file for the windows sysroot which avoids issues with filesystem case-sensitivity. | |
| `llvm.env` | clang/lld-link | Configures clang/lld-link using `CFLAGS`/`CXXFLAGS`. | [clang-cl and lld-link](./llvm.md) |
| `toolchain.cmake` | cmake | Provides a cross-compilation toolchain for CMake using llvm. | [CMake](./cmake.md) |
| `cmake.env` | cmake | Configures cmake using `toolchain.cmake` and `vfsoverlay.json`. | [CMake](./cmake.md) |
| `rustc.env` | rustc/cargo | Configures rustc using `RUSTFLAGS`/`CFLAGS`/`CXXFLAGS` | [cargo and rustc](./cargo.md) |

`nixwin setup --cmake` additionally writes a lockfile-aware toolchain *wrapper* to
`$NIXWIN_DATA/toolchain.cmake`; see [CMake](./cmake.md#the-lockfile-aware-wrapper).

### wine

`nixwin setup --wine` exports `WINEPATH` pointing at the VCR debug libraries of the *default* sysroot, one directory per installed architecture, e.g. `$NIXWIN_SYSROOT/VCR/<version>/bin/x86_64`. Wine searches `WINEPATH` for DLLs, so nothing is copied and no wine prefix is created or modified. The default sysroot must be installed with the `debug` feature, which is what provides the VCR debug libraries (*see [what's included?](#whats-included)*).

Like the other setup exports, the `WINEPATH` export is written to your shell rc in its own block (`# >>> nixwin (wine) >>>`) and is only picked up by shells started afterwards.

!!! important
    The export is scoped to the shell session it is sourced in. Processes which are not started from that shell, such as a graphical session or a service which was already running when `nixwin setup` ran, do not see it. Start them from a shell, or export `WINEPATH` yourself.

!!! note
    Nixwin no longer copies DLLs into a wine prefix. Prefixes which were populated by earlier nixwin versions keep those copies; they are not cleaned up automatically, so remove them yourself if you want the prefix to only use the sysroot's libraries.

The [wine guide](./wine.md) covers running debug binaries, including how to set
`WINEPATH` for a single command or a CI job.

## paths

The full list of variables nixwin reads is in
[configuration and environment](./configuration.md). The three that matter
most:

| variable | description | default |
|---|---|---|
| `NIXWIN_SYSROOT` | path to the default sysroot, a symlink into `sysroots/` | `$NIXWIN_DATA/sysroot` |
| `NIXWIN_SYSROOTS` | path to the tagged sysroot directories | `$NIXWIN_DATA/sysroots` |
| `NIXWIN_DATA` | path to the data directory | `$HOME/.local/share/nixwin` |

`nixwin setup` writes the paths it resolved for this machine into your shell rc (`~/.zshrc` or `~/.bashrc`), no flag required:

```sh
# >>> nixwin >>>
export NIXWIN_SYSROOT="$HOME/.local/share/nixwin/sysroot"
export NIXWIN_DATA="$HOME/.local/share/nixwin"
export NIXWIN_CACHE="$HOME/.cache/nixwin"
# <<< nixwin <<<
```

`CMAKE_TOOLCHAIN_FILE` is written to a separate block (`# >>> nixwin (cmake) >>>`) and only by `nixwin setup --cmake`.

!!! warning
    Both blocks are rewritten on every `nixwin setup` run, so a changed `--data-dir` or `--cache-dir` is picked up. Hand edits between the markers are lost on the next run, keep your own exports outside of the block. If nixwin cannot detect a shell rc it prints the exports to add to your profile instead.

!!! note
    The exports reflect the last `nixwin setup` run. Passing `--data-dir`/`--cache-dir` to an individual command still wins for that command, but the shell is not updated until setup runs again.

## cache directory layout

```sh
$NIXWIN_CACHE/
    VC/Tools/MSVC/${CRT_VERSIONS...}/
        include/
        lib/${ARCHS...}/
    VCR/${VCR_VERSIONS...}/bin/${ARCHS...}/
    Windows Kits/10/
        Include/${SDK_VERSIONS...}/
        Lib/${SDK_VERSIONS...}/{ucrt,um}/${ARCHS...}
```

## data directory layout

```sh
NIXWIN_CACHE_MSVC=$NIXWIN_CACHE/VC/Tools/MSVC/
NIXWIN_CACHE_VCR=$NIXWIN_CACHE/VCR
NIXWIN_CACHE_SDK=$NIXWIN_CACHE/Windows Kits/10/
$NIXWIN_DATA/
    sysroots/$TAG/
        VC/Tools/MSVC/$CRT_VERSION/
            include/            # ln -s $NIXWIN_CACHE_MSVC/$CRT_VERSION/include
            lib/${ARCHS...}/    # ln -s $NIXWIN_CACHE_MSVC/$CRT_VERSION/lib/${ARCHS...}
        VCR/$VCR_VERSION/bin/${ARCHS...}/ # ln -s $NIXWIN_CACHE_VCR/$VCR_VERSION/bin/${ARCHS...}
        Windows Kits/10/
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