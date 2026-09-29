# Configuration and environment

`nixwin` resolves every path from environment variables, then from the global
flags, and stores per-machine defaults in a single JSON file. This page lists
all of them.

## Environment variables

| variable | description | default |
|---|---|---|
| `NIXWIN_DATA` | data directory: installed sysroots and the machine configuration | `$HOME/.local/share/nixwin` |
| `NIXWIN_CACHE` | shared component cache, symlinked into every sysroot | `$HOME/.cache/nixwin` |
| `NIXWIN_SYSROOTS` | directory holding the tagged sysroots | `$NIXWIN_DATA/sysroots` |
| `NIXWIN_SYSROOT` | the *default* sysroot, a symlink to one of the tagged sysroots | `$NIXWIN_DATA/sysroot` |
| `NIXWIN_CACHE_MSVC` | cached MSVC toolchain versions | `$NIXWIN_CACHE/VC/Tools/MSVC` |
| `NIXWIN_CACHE_SDK` | cached Windows SDK versions | `$NIXWIN_CACHE/Windows Kits/10` |
| `NIXWIN_CACHE_VCR` | cached VCR redistributable versions | `$NIXWIN_CACHE/VCR` |

An empty value is treated as unset, so `NIXWIN_DATA=` in the environment falls
back to the default rather than resolving to the current directory.

`nixwin` exports two more variables, but writes rather than reads them:

| variable | written by | points at |
|---|---|---|
| `CMAKE_TOOLCHAIN_FILE` | `nixwin setup --cmake` | the generated toolchain wrapper in `$NIXWIN_DATA` |
| `WINEPATH` | `nixwin setup --wine` | the VCR debug libraries of the default sysroot, one entry per architecture |

### What `nixwin setup` writes

`nixwin setup` resolves the paths for this machine and writes them to your
shell rc (`~/.zshrc` or `~/.bashrc`, whichever matches `$SHELL`), inside a
managed block:

```sh
# >>> nixwin >>>
export NIXWIN_SYSROOT="$HOME/.local/share/nixwin/sysroot"
export NIXWIN_DATA="$HOME/.local/share/nixwin"
export NIXWIN_CACHE="$HOME/.cache/nixwin"
# <<< nixwin <<<
```

`CMAKE_TOOLCHAIN_FILE` and `WINEPATH` get their own blocks,
`# >>> nixwin (cmake) >>>` and `# >>> nixwin (wine) >>>`, written only by the
flag that asks for them.

!!! warning "The managed blocks are rewritten on every `nixwin setup` run"

    Hand edits between the markers are lost on the next run, so a changed
    `--data-dir` or `--cache-dir` is picked up but your own edits are not. Keep
    your exports outside the block. If a block has only one of its two markers,
    `nixwin` refuses to touch the file rather than eating unrelated lines; fix
    or delete the orphaned marker yourself.

The exports reflect the *last* `nixwin setup` run. Passing `--data-dir` or
`--cache-dir` to an individual command still wins for that command, but the
shell is not updated until `nixwin setup` runs again.

## Global flags

Available on every command:

| flag | description | overrides |
|---|---|---|
| `--data-dir <PATH>` | data directory for this invocation | `NIXWIN_DATA` |
| `--cache-dir <PATH>` | cache directory for this invocation | `NIXWIN_CACHE` |
| `--config <PATH>` | read the install configuration from a specific lockfile | `$CWD/.nixwin.json` |

`--data-dir` and `--cache-dir` only affect the command they are passed to.
`--config` selects which lockfile `nixwin install` resolves from, and applies to
the other commands that read one.

## `nixwin config`

The machine configuration lives at `$NIXWIN_DATA/config.json`. `nixwin config`
reads and writes it, and it is separate from both the per-sysroot configuration
and the project lockfile (see [lockfiles](lockfiles.md)).

```sh
# print the whole configuration
nixwin config
# read one key
nixwin config default.tag
# set the default sysroot, which repoints the $NIXWIN_SYSROOT symlink
nixwin config default.tag 17
# remove a key again
nixwin config --unset default.channel
```

| key | type | description |
|---|---|---|
| `default.tag` | sysroot tag | the sysroot `$NIXWIN_SYSROOT` points at |
| `default.arches` | list | architectures to install when no flag is given: `x86`, `x86_64`, `aarch`, `aarch64` |
| `default.variants` | list | SDK/CRT variants: `desktop`, `onecore`, `store`, `spectre` |
| `default.features` | list | features: `debug`, `atl` |
| `default.manifest` | number | Visual Studio manifest version to resolve from |
| `default.channel` | string | manifest channel, e.g. `release` or `pre` |
| `cmake` | boolean | whether the CMake integration is configured; written by `nixwin setup --cmake` |
| `wine` | boolean | whether the wine integration is configured; written by `nixwin setup --wine` |
| `tpl.<name>` | path | override the source of a generated file, see below |

!!! note "The architecture key is spelled `default.arches` on the command line"

    It is stored as `archs` in `config.json`. Use `default.arches` with
    `nixwin config`; if you hand-edit the JSON, the key is `default.archs`'s
    target, `archs`.

### Template overrides

`nixwin` renders five files into each sysroot from handlebars templates built
into the binary. `tpl.<name>` replaces the source of one of them, which is how
you would adapt a generated toolchain to a project that needs an extra flag:

| name | generates |
|---|---|
| `tpl.toolchain` | `<sysroot>/toolchain.cmake` |
| `tpl.cmake` | `<sysroot>/cmake.env` |
| `tpl.rustc` | `<sysroot>/rustc.env` |
| `tpl.llvm` | `<sysroot>/llvm.env` |
| `tpl.cmake-wrapper` | the `CMAKE_TOOLCHAIN_FILE` wrapper written by `nixwin setup --cmake` |

```sh
nixwin config tpl.toolchain /path/to/custom/toolchain.cmake
```

The override path is read when a sysroot is installed or rebuilt, so re-run
`nixwin install` (or `nixwin rm` with component flags, which rebuilds) to apply
it.

## Resolution order

When `nixwin install` decides what to install, values are combined as follows:

- **scalars** (`tag`, `manifest`, `channel`, `sdk`, `crt`) take the first value
  found in: command-line flag, [lockfile](lockfiles.md), installed sysroot
  configuration, `default.*` in `config.json`, builtin default.
- **lists** (`archs`, `variants`, `features`) are the *union* of the installed
  sysroot configuration, the lockfile and the flags. The `default.*` machine
  defaults apply only when that union is empty.

!!! important "Lists union, so a removal can come back"

    `nixwin rm -a x86` removes an architecture from the sysroot's configuration,
    but a `.nixwin.json` that still lists `x86` re-adds it on the next
    `nixwin install`. Update the project lockfile in the same change as the
    removal.
