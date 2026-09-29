# AGENTS.md

How AI agents should work in this repository. Read this before making changes.

## What this repository is

`nixwin` is a Rust CLI that builds Windows sysroots on unix hosts for
cross-compiling C, C++ and Rust. It is a single crate, no workspace, with:

- `src/` — the CLI, path resolution, configuration and template rendering
- `resources/templates/` — handlebars templates emitted into each sysroot
- `tests/` — integration tests, one file per feature area
- `docs/` — the documentation site content, published with Zensical
- `examples/` — one cross-compiled program per toolchain, each with its own CI file
- `install.sh` — the user-facing installer

## Code style

Rust, formatted with `rustfmt` defaults. There is no `rustfmt.toml`; do not add
one as a drive-by change.

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test` must all pass before you consider work done.
- Prefer existing dependencies over adding new ones. The runtime set is small
  (`anyhow`, `clap`, `handlebars`, `indicatif`, `rayon`, `serde`, `serde_json`,
  `xwin`); `tempfile` is the only dev-dependency.
- Errors are `anyhow::Result` with `.with_context(...)` or `.context(...)` at the
  point where context is known. Do not introduce `unwrap()`/`expect()` outside
  tests.
- Commands are `clap` derive types in `src/cmd/`, one module per command area,
  dispatched from `src/main.rs`.
- Doc comments state what a field or function is for and what the user sees.
  `src/paths.rs` is the reference for how paths and their environment variables
  are documented.
- Comment *why*, not *what*. If a line needs a comment to say what it does, it
  probably needs a rewrite instead.
- Keep user-facing text lowercase and without trailing punctuation, matching the
  existing `println!` output.

Shell scripts (`install.sh`, the example `build.sh` files) are bash with
`set -euo pipefail`, tab indentation, and `[ "$x" = "y" ]` style tests.

Documentation is plain markdown. The site additionally understands `!!! note`
admonitions, so use those rather than GitHub `> [!NOTE]` callouts.

## Commits

Conventional commits, concise. One logical change per commit.

```
<type>(<scope>): <summary>
```

Types in use: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `build`, `ci`.
Scopes are areas: `rm`, `setup`, `install`, `emit`, `paths`, `config`, `docs`,
`examples`, `site`.

The summary is imperative, lowercase, under 72 characters, and does not end with
a period. The subject line must stand alone — `git log --oneline` should read as
a changelog.

```
feat(rm): remove single components from a sysroot
docs(site): add the cargo and wine integration guides
fix(paths): treat an empty NIXWIN_DATA as unset
```

## Branching

Branch names are `<scope>/<summary-slug>`, a scope followed by a short kebab-case
slug of the summary:

```
rm/selective-component-removal
site/integration-guides
paths/empty-env-var
```

The scope matches the commit scope. Create the branch before the first commit,
unless you are explicitly told to work on an existing one. Do not commit directly
to `main`.

## Do not modify this file

Agents must not edit `AGENTS.md` themselves. Its contents are a human's decision
about how work is done in this repository, and an agent rewriting the rules it is
being held to is a way for those rules to erode unnoticed.

The single exception is when a human explicitly asks for it, in the conversation
and for a stated reason. "Update AGENTS.md to cover the new test helper" is a
request; noticing that a section feels out of date is not. If you think a rule
here is wrong, say so in your summary and let the human decide.

## Finishing work

When the work is complete, present a pull request description filled in from the
matching template in `.github/PULL_REQUEST_TEMPLATE/`:

| change | template |
|---|---|
| repairing broken behaviour | `bug_fix.md` |
| new capability | `feature.md` |
| restructuring with no behaviour change | `refactor.md` |
| docs, CI, housekeeping | `pull_request_template.md` |

Fill in every section that applies and delete the guidance comments. Choose the
template by what the change *is*, not by how much text it produced.

Then **stop**. Present the description and wait. Do not open the pull request,
do not push the branch, and do not create the PR through `gh` or the GitHub API
— opening it is the human's action, and a description they can read and edit
before it exists is worth more than one that is already live.

Report honestly. If a test fails, a command could not run, or a claim could not
be verified, say so in the summary rather than leaving it for review to discover.
If you found a problem outside the scope of the work, report it instead of
silently fixing or silently ignoring it.
