# Fast development with two humans and parallel agents

Project owner: [@crooruhe](https://github.com/crooruhe). For this foundation PR,
@AstralDrift is the responsible operator and @crooruhe is the human reviewer.
Both teammates are listed in CODEOWNERS so future work can receive reciprocal review.

## Local loop

Install Rust using [rustup](https://rustup.rs/) and Python 3.11 or later. The checked-in
toolchain is Rust 1.98.1 with rustfmt and clippy. There are no external Rust dependencies
in the initial workspace. Run from a clean task worktree:

```sh
./scripts/check --quick             # format, script tests, cargo check
cargo test -p hearth                # focus on the component being changed
./scripts/check                     # identical full baseline in CI
cargo run -p hearth -- --help
```

The full baseline runs script tests, format, clippy with warnings denied, and workspace
tests, with Cargo.lock enforced. Neither script edits source. Hooks are optional:

```sh
git config --local core.hooksPath .githooks
```

The pre-commit hook checks formatting of the working tree, not only staged content.
It is a convenience, not an enforcement boundary; full checks still run in CI.
If committing is blocked by unrelated unstaged formatting, isolate the task worktree.

## Parallel implementation, serialized integration

Assign one human operator and one issue or bounded subtask to each branch. The operator
owns verification, review, merge, and recovery, regardless of who typed the code.
Use one worktree per agent, for example:

```sh
git fetch origin
git worktree add -b issue-3-core ../hearth-issue-3 origin/master
```

Use GitHub issues and draft PRs for scope and status; do not create a second tracker.
Agree on shared contracts in #3 before dependent agents invent their own. Assign a
single integration owner for Cargo.toml, Cargo.lock, shared types, and CI. Tell peers
about interface changes before coding against them. Keep each PR independently useful
and revertible; split unrelated cleanup. Land workspace changes before dependent crates.

Before handoff run full checks, demonstrate the changed behavior, and get one reviewer
with fresh context to follow [REVIEW.md](../REVIEW.md). Supply the issue, base/head SHAs,
relevant callers, and real evidence. Record findings and their resolutions in the PR.
After fixes, review the affected diff; stop automated loops after three rounds.
No paid cloud AI reviewer or credential-bearing agent workflow is required.

## Human attention follows risk

| Risk | Examples | Human review |
|---|---|---|
| Routine | Copy, layout, non-authoritative docs | Operator verifies evidence |
| Behavioral | Triage, MCP integration, retained state | Operator checks behavior and AI findings |
| Boundary-sensitive | Policy, broker, sandbox, secrets, receipts, approvals, CI | Other teammate reviews the exact revision |

These are review-depth guidelines, not an automated classifier. The existing GitHub
ruleset requires one teammate approval on **every** PR; this setup preserves that
stronger requirement. Sensitive paths additionally request owners in CODEOWNERS.
New pushes dismiss approvals; unresolved review threads block merging. Neither an
agent's review nor its operator's self-approval replaces the teammate approval.
Humans merge; agents prepare changes and evidence. Do not grant agents merge access
as a workaround. This PR does not implement conditional self-merge or auto-merge.

Required proof follows AGENTS.md's verification table. In particular, a mocked deny
does not prove isolation, a successful build does not prove terminal restoration,
and passing unit tests do not prove a full no-agent demo. Ubuntu 24.04 and Arch
userspace CI run the same baseline. Primary targets are native Ubuntu 24.04 and
Arch Linux; develop, rehearse, and record the demo on native Linux. Container CI
does not establish native sandbox enforcement. Real Linux sandbox tests must land
with #9 and run on native Ubuntu and Arch; unavailable required backends must fail
rather than skip. #15 must probe actual isolation prerequisites there, and #14 must
rehearse seed/reset and the no-agent demo on the native demo machine. WSL compatibility
is optional follow-up, not a hackathon acceptance gate; test it separately before
claiming support. macOS support and CI are outside scope. Current CI does not claim
sandbox or WSL support.

## GitHub checks and activation

CI runs for PRs, pushes to master, and merge groups. It caches builds by OS, architecture,
toolchain and manifests, cancels superseded runs only for the same PR, and always reports
the stable `CI` aggregate. Workflow-level path filters are intentionally absent so a
required check cannot disappear on documentation-only PRs. The Ubuntu 24.04 and Arch Linux check jobs must
both pass. Arch runs the official rolling `archlinux:base-devel` container with a full
package update; this tests Arch userspace on the hosted Ubuntu kernel, not native
Arch kernel isolation. Its build cache is separate from Ubuntu’s. CodeQL analyzes Rust and Actions on PRs, master, merge groups, and weekly.
Rust analysis uses build mode `none`; generated code may receive less complete analysis.
The workflow excludes `rust/hard-coded-cryptographic-value` because fixed salts in the
game renderer produce false positives. This rule is disabled for all Rust code in the
repository. All other default queries remain active.
CodeQL complements runtime boundary tests. Actions are pinned to immutable commits.

All ordinary workflows use read-only contents permissions. Only CodeQL analysis gets
security-events write permission. There is no pull_request_target execution or self-hosted
runner. Never feed untrusted PR code to a privileged follow-up workflow.

An administrator can preview and apply settings after the exact PR revision has passed
`CI`, `CodeQL (rust)`, and `CodeQL (actions)`:

```sh
python3 scripts/configure-github.py --ref <verified-commit-sha>
python3 scripts/configure-github.py --ref <verified-commit-sha> --apply
```

The command preserves existing rules and bypass lists, never lowers the approval count,
binds required checks to the verified GitHub Actions app, requires owner review, dismisses
stale approvals, and requires resolved threads. Existing CodeQL alert thresholds stay
in force. Existing unrelated rules, including GitHub Code Quality and Copilot review,
remain untouched. Preview the payload and investigate any unavailable existing service;
do not remove a protection just to obtain a green PR.

After this PR merges and master checks pass, activate the queue with the **current master
commit SHA**. The script refuses activation before workflows with merge_group exist on
the default branch and all three checks pass on that SHA:

```sh
python3 scripts/configure-github.py --ref <current-master-sha> --queue
python3 scripts/configure-github.py --ref <current-master-sha> --queue --apply
```

The queue tests integrations with the latest master, squashes one PR per merge, and
builds at most two queue entries concurrently. Before activation, branches must be
up to date and pass checks; coordinate merges through one human integrator. Re-run
the command to reconcile settings; it is idempotent. Repository settings live outside
Git: reverting this PR does not revert them. Save the prior ruleset before applying.

## Recovery and learning

Use focused squash merges. To undo a normal squash merge, prepare a revert PR:

```sh
git fetch origin
git switch -c revert-regression origin/master
git revert <squash-commit-sha>
./scripts/check
```

The operator opens the revert PR and gets normal checks/review; never force-push master.
Persistent state needs an explicit rollback plan. Avoid tying unrelated features to one
commit. Feature toggles can help incomplete UI, but must never disable enforcement.

The operator who lands a regression owns fixing or reverting it. Capture the incident in
the relevant GitHub issue/PR and add a regression test; add guidance only when tooling
cannot catch the class of failure. Watch merge wait, repeated review rounds, and reverts
using GitHub history. Do not build a metrics platform during the hackathon.

## Sources and limits

- [Official Arch Linux container and update policy](https://github.com/archlinux/archlinux-docker)
- [Merge queue and merge_group](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/configuring-pull-request-merges/managing-a-merge-queue)
- [Secure Actions usage](https://docs.github.com/en/actions/reference/security/secure-use)
- [CodeQL Rust build mode](https://docs.github.com/en/code-security/reference/code-scanning/codeql/build-options-for-compiled-languages)
- [Ruleset API](https://docs.github.com/en/rest/repos/rules)

The workspace CLI is only a help/version entry point. #3 owns core contracts, #4 owns
the TUI, #5 owns demo clusters, and later issues own enforcement and demo behavior.
This foundation does not complete those issues.
