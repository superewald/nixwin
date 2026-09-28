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
| option |  description | default |
|---|---|
| `--cmake` | Set `CMAKE_TOOLCHAIN_FILE` to a wrapper which auto-selects the sysroot toolchain when `.nixwin.json` is present. | false |
| `--wine` | Export `WINEPATH` in your shell rc, so wine resolves the VCR debug libraries of the default sysroot. | false |

## install

**Synopsis**: `nixwin install [MANIFEST_VERSION] [OPTIONS]`

Install pre-configured [windows sysroots](./sysroots.md) from VS package store.

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

| option |  description | default | values |
|---|---|---|---|
| `-t`, `--tag` | Name of the windows sysroot | manifest version *(see `--manifest`)* | name must use valid path characters |
| `-a`, `--archs` | Target architectures to include. | Host system architecture | `[x86,x86_64,aarch,aarch64]` |
| `-f`, `--features` | Features to include. | `debug` unless CI | `[debug,atl]` |
| `--variants` | SDK/CRT variants to include. | `desktop` | `[desktop,onecore,store,spectre]` |
| `--manifest` | Visual Studio manifest version to use. | `MANIFEST_VERSION` or `17` | `[16,17,18]` |
| `--channel` | Visual Studio manifest channel to use. | `release` | eg `release`, `pre` |
| `--sdk` | Use a specific WindowsSDK version. | - | eg `10.0.28000.0` |
| `--crt` | Use a specific CRT version (this also determines the VCR version). | - | eg `14.40.33807` |
| `-d`, `--default` | Set this windows sysroot as user default (*see [config](#config)*) | - | |
| `-l`, `--lock` | Path to the lockfile to write. If not specified no lockfile is generated. | `$CWD/.nixwin.json` | valid path |

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

> [!NOTE]
> The architecture key is spelled `default.arches` on the command line but is stored as `archs` in `config.json`. Use `default.arches` with `nixwin config`; hand-editing the JSON requires `archs`.

## ls

**Synopsis**: `nixwin ls`

List the installed windows sysroots.

## rm

**Synopsis**: `nixwin rm [TAG]`

Removes a windows sysroot from disk. If `TAG` is omitted the default sysroot is removed. Removing the default sysroot also clears the configured default tag.

`rm` takes no options: to drop an architecture, feature or variant from a sysroot, install it again without that component.

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