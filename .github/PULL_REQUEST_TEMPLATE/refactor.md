## What changed

<!--
The restructuring, as a reader of the code would notice it. Name the modules and
types involved, not the diff statistics.
-->

Closes #

## Why

<!--
What was wrong with the previous shape: the duplication, the coupling, the
concept that was in the wrong place. This is the argument for the churn, so make
it properly.
-->

## Behaviour is unchanged

<!--
This is the central claim of a refactor. State how you know.

A refactor that quietly changes behaviour is a bug fix wearing a disguise, and
reviewers cannot tell the difference without being told.
-->

- [ ] No observable behaviour changes, and I am confident
- [ ] Behaviour does change slightly — describe it below and treat it as a bug fix

<!-- If the second box is checked, list exactly what changes. -->

## Coverage

<!--
- [ ] Pure move: the diff is moves and deletions only
- [ ] Structure change: types, traits, module boundaries
- [ ] Test-only: mocks, fixtures, helpers
-->

## How it was verified

<!--
Tests run before and after, and any output that should be byte-identical. For a
move, say that the test count is unchanged.
-->

## Checklist

- [ ] `cargo test` passes, with the same number of tests as before
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean
- [ ] No dead code, commented-out code or re-export left behind
- [ ] The site builds without link warnings, if any file or link moved: `zensical build --clean`
