# wine

A sysroot contains the *link-time* libraries. Running a Windows binary under
wine additionally needs the CRT **runtime** DLLs, and the debug build of those
is only present when the sysroot was installed with the `debug` feature.

`nixwin setup --wine` points wine at them through `WINEPATH`.

```sh
nixwin install 17 --features debug --default
nixwin setup --wine
wine ./build/app.exe
```

## How it works

`WINEPATH` is a list of directories wine searches for DLLs. The export nixwin
writes contains one entry per architecture the *default* sysroot was installed
for, pointing at that architecture's VCR debug runtime:

```sh
# >>> nixwin (wine) >>>
export WINEPATH="$NIXWIN_SYSROOT/VCR/14.40.33807/bin/x86_64;$NIXWIN_SYSROOT/VCR/14.40.33807/bin/x86"
# <<< nixwin (wine) <<<
```

Three things follow from that:

- **Nothing is copied.** The DLLs stay in the sysroot; wine is told where to
  look. No wine prefix is created, and no prefix is modified.
- **The paths are resolved through `$NIXWIN_SYSROOT`**, not through the cache,
  so the value follows whichever sysroot is currently the default. Change the
  default with `nixwin config default.tag` and the export picks it up on the
  next `nixwin setup --wine`.
- **Entries are separated by `;`**, which is what wine expects on every host
  platform, not the `:` a unix shell would use.

The `debug` feature is what provides these libraries, so `nixwin setup --wine`
fails with a clear message if the default sysroot was installed without it.

## The export is scoped to your shell session

!!! important "`WINEPATH` is only visible to processes started from that shell"

    The export lives in a managed block in your shell rc, so it takes effect
    when a shell *reads* the file. It is scoped to that shell session, and
    only descendants of the shell inherit it.

    In practice this means:

    - open a new terminal, or run `source ~/.zshrc` (`~/.bashrc`), after
      `nixwin setup --wine`;
    - a GUI session, an editor started from the desktop environment, or a
      service that was already running when `nixwin setup` ran will **not**
      see the variable — start it from a shell, or export `WINEPATH` yourself;
    - a non-interactive shell, as used by many CI runners, does not read your
      rc at all. Export it explicitly in the job.

    You can check what a shell currently sees with `echo "$WINEPATH"`.

## Doing it by hand

Prefer to set the variable in a single command or a CI job rather than
persist it:

```sh
export WINEPATH="$NIXWIN_SYSROOT/VCR/$VCR_VERSION/bin/x86_64"
wine ./build/app.exe
```

The version is in the sysroot's [lockfile](lockfiles.md) as `vcr`, or you can
list the directory:

```sh
ls "$NIXWIN_SYSROOT/VCR"
```

For a release build none of this is needed — wine ships its own builtin
`msvcp`/`vcruntime` DLLs, which is why a release binary usually runs without a
`WINEPATH` at all. The export exists for the *debug* CRT, which wine does not
provide.

!!! note "Prefixes populated by older nixwin versions"

    Earlier versions copied DLLs into the wine prefix. Those copies are still
    there and still win over `WINEPATH`, so a prefix that was populated once
    keeps using them. nixwin does not clean them up; delete them yourself if
    you want the prefix to use the sysroot's libraries.
