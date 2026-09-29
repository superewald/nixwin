## Summary

<!-- What does this change do, and why? One or two sentences. -->

## Type of change

<!-- Delete the ones that do not apply. -->

- [ ] Bug fix (a broken behaviour is repaired) → use `bug_fix.md`
- [ ] Feature or new capability → use `feature.md`
- [ ] Refactor, with no behaviour change → use `refactor.md`
- [ ] Documentation or repository housekeeping → this template is fine

## Related issue

<!-- Closes #123, or "none". -->

## How it was verified

<!--
What you ran, and what you observed. Prefer a command and its output over a
description of it. If the change needs a sysroot, say which one
(`nixwin inspect <TAG>`) so it can be reproduced.
-->

## Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] Documentation touched by this change is updated, and relative links resolve
- [ ] The site builds without link warnings: `zensical build --clean`
