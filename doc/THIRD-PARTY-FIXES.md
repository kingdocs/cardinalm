# Third-party engine fixes: sync policy

> This document is part of the repository so the policy survives a fresh clone.
> A longer, machine-specific maintenance copy lives locally as `UPSTREAM-SYNC.md`.

## The three repositories

| Role | Repository | Relationship |
| --- | --- | --- |
| **Only base** | `upstream` = cardisoft/cardinal | rebase onto it; read-only, never push |
| **Our layer** | `origin` = kingdocs/cardinalm | own features + ported fixes, all our own commits |
| **One-time donor** | `franciscoxc/cardinal` | **no remote, no tracking branch, no merge** |

`franciscoxc/cardinal` is another personal fork (based on upstream v0.1.23,
self-versioned 0.5.2, 66 commits / 105 files, Developer ID signed and notarized).
We took **only the engine bug fixes**, by hand. The relationship ends there.

> **Upstream updating while that fork does not is a non-issue.** We never wait
> for it and never check its state. Once ported, the code is ours and rebases
> onto new upstream releases together with everything else.

## Why we do not track it

1. It has diverged from upstream: 66 commits, a rewritten content-search
   interior (`content_snippet`, `SKIPPED_CLOUD`, `content_candidates` machinery),
   its own signing/notarization and a full UI redesign. Making it an upstream
   or merging it would effectively replace the product.
2. Its new infrastructure is not present here, so later fixes would become
   progressively harder to take in isolation.
3. Upstream remains the nominal mainline for version numbers, cache format and
   release tags, even though it has been quiet since v0.1.23 (2026-03).

If we ever want another fix from that fork, repeat the "evaluate -> hand-port"
cycle, re-checking the base difference each time. **Do not automate it and do
not create a dependency.** If upstream were abandoned entirely and that fork
became the de-facto mainline, switching bases would be a separate decision.

## What to do when upstream updates

Same as always, regardless of that fork's state:

```bash
git fetch upstream
git log --oneline --no-merges master..upstream/master
git rebase upstream/master          # all our commits replay, including the ports
cargo test -p fswalk
cargo test -p search-cache
cargo test -p cardinal-syntax
```

Two things changed:

- Conflict hotspots grew from one file to four: `search-cache/src/cache.rs`,
  `search-cache/src/query.rs`, `cardinal-syntax/src/lib.rs`, and the tests.
- One new decision: **has upstream fixed the same bug itself?** Three of the
  fixes were reported upstream (#229 / #230 / #231), so it is possible.

Decide by landing function + regression test, not by eyeballing:

| Upstream state | Action |
| --- | --- |
| Fixed it, semantically equivalent | **Drop our port** (revert that commit or take upstream in the conflict), follow upstream |
| Fixed it, different implementation | **Stop and ask**, same as PR #152 — do not choose unilaterally |
| Not fixed | Keep the port |

## Ledger (basis for dropping a port)

| # | Fix | Source commit | Landing point here | Upstream issue | Drop condition |
| --- | --- | --- | --- | --- | --- |
| 1 | FS event batches no longer discarded; skip watch root | `ac431d69` | `cache.rs::handle_fs_events` | #229 | Upstream no longer returns early on a single rescan event |
| 2 | Content search does not open iCloud dataless placeholders | `8bc9bae5` | `query.rs::node_content_matches` | #231 | Upstream checks `SF_DATALESS` before opening a file |
| 3 | `content:` runs after every other filter (was 60x by word order) | `ec682553` | `cardinal-syntax/src/lib.rs::reorder_by_priority` | #230 | Upstream gives `FilterKind::Content` its own highest priority |
| 4 | Never read `/dev` or anything no longer a regular file | `6cf78bdb` | `query.rs::node_content_matches` | -- | Upstream validates type / `/dev` before reading |
| 5 | Cancelled content search stops dispatching candidates at once | `fe8e88e2` | `query.rs::content_candidates` | -- | Upstream checks cancellation on the dispatching side |

Fixes 1 and 3 were near-direct ports (upstream-shaped code). Fixes 2, 4 and 5
were **hand-adapted**: this fork is on the original v0.1.23 interior, without
`unread_reason`, `content_snippet`, `SKIPPED_CLOUD` or `content_candidates`.
Only the guard logic was taken — no skip counters, no context column, no new deps.

## Source markers and self-check

Every ported hunk carries `[port:franciscoxc/<sha>]`, distinct from the Chinese
comments used by our own features. After a rebase:

```bash
grep -rnF "[port:franciscoxc/" --include=*.rs . | grep -v /target/ | sort
```

Baseline: **19 markers across 5 source SHAs** (measured 2026-11-11). Counts may
change, but no SHA may disappear.

Every fix has a regression test — these are the detectors for "did upstream fix
it too?":

```bash
cargo test -p search-cache --lib ported_fixes
cargo test -p search-cache --lib a_rescan_event_does_not_discard
cargo test -p cardinal-syntax --test optimizer_metadata_tail content_filter_runs_after
```

Known limitation: a plain file cannot be turned into a real iCloud placeholder
with `chflags dataless` on a normal macOS machine (only the cloud daemon sets
that flag), so the end-to-end dataless test prints `SKIP`. The flag-bit logic is
covered unconditionally by `dataless_flag_is_recognised`.

## Release convention

- Ported fixes are ordinary commits of ours; the commit message lists all five
  source SHAs and issue numbers so any one can be reverted precisely.
- Functional changes bump the last number of `v<upstream-version>-fork.<n>`
  and get a new Release. Signing-only or docs-only changes replace the asset on
  the existing tag instead of creating a near-identical Release.
- Re-sign after every rebuild with `scripts/sign-self-signed.sh`.
