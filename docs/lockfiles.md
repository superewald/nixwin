# Nixwin Lockfiles

A lockfile is a `.nixwin.json` file which pins a sysroot to a fully resolved set of components: its `tag`, `manifest`, `channel`, `archs`, `variants`, `features`, `sdk`, `crt` and `vcr`. Commit a lockfile to version control to make installs reproducible across machines and CI runs.

Lockfiles are written to two places:

- `$CWD/.nixwin.json` — the project lockfile, written on demand with `nixwin install --lock`
- `$NIXWIN_DATA/sysroots/$TAG/.nixwin.json` — the configuration of an installed sysroot, emitted on every install

This is separate from the machine configuration (`$NIXWIN_DATA/config.json`, see [`nixwin config`](./configuration.md#nixwin-config)), which holds per-machine install defaults and integration flags rather than pinned versions.

## creating a lockfile

```sh
# create .nixwin.json for the VS 16 sysroot in the current directory
nixwin install 16 --lock
# create .nixwin.json in another project directory
nixwin install 16 --lock /path/to/project
# write to an explicit file instead
nixwin install 16 --lock path/to/custom.lock.json
```

`--lock` takes an optional path. A directory receives the default `.nixwin.json` name, any other path is written verbatim.

## how lockfiles are used

| consumer | behavior |
|---|---|
| `nixwin install` | reads `$CWD/.nixwin.json` unless `--config <PATH>` points at another lockfile |
| `--config <PATH>` | global flag, installs from the lockfile at an explicit path |
| `nixwin setup --cmake` | the generated `CMAKE_TOOLCHAIN_FILE` wrapper scans `${CMAKE_CURRENT_SOURCE_DIR}` and its own directory for `.nixwin.json`, and uses the `tag` therein to include `$NIXWIN_DATA/sysroots/$TAG/toolchain.cmake`. Falls back to `$NIXWIN_SYSROOT` and then `$NIXWIN_DATA/sysroot` when no lockfile is found. See [the lockfile-aware wrapper](./cmake.md#the-lockfile-aware-wrapper). |
| CI | hash the lockfile into your cache key so the cache is invalidated when component versions change |

When resolving an install, lockfile values are combined with flags as follows:

- scalars (`tag`, `manifest`, `channel`, `sdk`, `crt`): flags > lockfile > installed sysroot configuration > machine defaults > builtin defaults
- lists (`archs`, `variants`, `features`): the union of the installed sysroot configuration, the lockfile and the flags. Machine defaults are only applied when that union is empty.