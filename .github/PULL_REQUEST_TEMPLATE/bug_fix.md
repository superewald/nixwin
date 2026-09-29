## What was broken

<!--
The defect: the observed behaviour, and the behaviour that is correct.
If this came from an issue, "Closes #123". Otherwise describe it here.
-->

## The fix

<!--
What you changed, and why that addresses the cause rather than the symptom.
If the fix changes an error message, a CLI surface or the on-disk layout, call
that out explicitly — it is a user-visible change and may need a changelog note.
-->

## Root cause

<!--
For a non-obvious cause, the short chain from cause to symptom. "Not obvious"
includes anything that needed more than one debugging step to find.
-->

## How it was verified

<!--
The failing case before the change, and the passing case after it, with the
commands used. A regression test is the strongest form of this — name it and
say what it asserts.
-->

## Regression coverage

- [ ] Added a test which fails before this change and passes after
- [ ] Existing coverage already covers this case, and I added no test — explain why below

<!-- If the second box is checked, say why the existing tests are sufficient. -->

## Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] Any behaviour change is reflected in the docs under `docs/`
