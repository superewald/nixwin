## What this adds

<!--
The capability, in terms of what a user can now do that they could not before.
Lead with that, not with the implementation.
-->

Closes #

## Why it is needed

<!--
The problem being solved, and who has it. Link the issue that requested it if
there is one. If you found the gap yourself, say what you were trying to do when
you hit it.
-->

## How to use it

<!--
A worked example, ideally one someone can run. A new command or flag needs the
exact invocation; a new configuration key needs the file and the key.
-->

```sh
# before: not possible
# after:
```

## Interface changes

<!--
Anything a user can observe: new commands, flags, config keys, environment
variables, files written, or changes to existing ones. Delete if there are none.
- [ ] New command or flag
- [ ] New or changed `nixwin config` key
- [ ] New or changed environment variable
- [ ] Changed file layout inside a sysroot
- [ ] No user-visible interface change
-->

## Design notes

<!--
The decision, the alternatives that were rejected, and what would make you
change your mind. Worth a paragraph; it saves the same discussion in review.
-->

## How it was verified

<!-- Commands run, and what you observed. -->

## Documentation

- [ ] Documented in `docs/`
- [ ] A new documentation page is linked from the `zensical.toml` nav
- [ ] The README mentions it if a user would look for it there

## Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] The site builds without link warnings: `zensical build --clean`
