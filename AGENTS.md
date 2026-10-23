# AGENTS.md

Hearth is one terminal for developers and their agents. Every tool is an extension. Agents use the same tools, under the same rules, as people. "Hearth" is a working name.

Stack: Rust host, any-language MCP extensions. Chosen 2026-10-23 after three independent reviews.
Fallback: if the team cannot debug Rust on build day, the same process-per-extension design ports to Python + Textual.

We build it for the Beast Code internal hackathon on Fri Oct 30 2026. Pitch: `docs/pitch.md`. Requirements and status: `TRACKER.md`.

## Hard rules

1. **No Beast Code data or code.** Event rule: "No Beast Code data or code - use a mock data set if you need to." Every cluster, log, incident, and image is a mock fixture: files in `fixtures/` or a local kind cluster with seeded failures. No real kubeconfigs, tokens, hostnames, or registry URLs. Do not name or recreate internal tools. A company touchpoint is a stub we define.
2. **Enforce in code and the OS, never in a prompt.**
3. **Default deny.** An extension touches only what its manifest declares.
4. **Agents and extensions never hold real secrets.** The broker injects them at the boundary.
5. **Every change leaves a receipt.**
6. **Model-agnostic.** Any provider plugs in through `hearth-providers`. No model API code outside that crate. No personal billing or account rules here.

## The demo is the acceptance test

We ask the agent to fix a broken mock service. It triages and finds a deployment issue. It tries to restart a pod it was never granted: the shell stops it. It tries to call an undeclared host: the shell blocks it. It tests its fix in the sandbox, a person approves, the check passes, and the receipt is on screen. 60 seconds.

If a change does not move this demo or a `TRACKER.md` row, stop and ask.

**Build order:** day 1, freeze manifest, policy, and receipt types. Day 2, the TUI shell: 3 modes, palette, suspend. Then a thin demo slice. Then widen. See `TRACKER.md`.

## Commands

No code yet. M0 adds these. Keep them stable.

- Run: `cargo run -p hearth`
- Preflight: `cargo run -p hearth -- doctor` (bwrap, user namespaces, kind)
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test all: `cargo test`
- Test one: `cargo test -p hearth-policy deny_pod_restart`
- Full check before done: `cargo fmt --check`, then lint, then test all. Use `cargo check` in the loop.

## Layout (target)

Cargo workspace. One binary, `hearth`.

- `crates/`: `hearth-core` (serde types: manifest, policy decision, receipt, config), `hearth-policy` (policy and approvals), `hearth-sandbox` (`SandboxBackend`), `hearth-broker` (Kubernetes, Docker, images, egress proxy, secrets), `hearth-providers` (`ModelProvider`), `hearth-tui` (modes, palette), `hearth` (binary, MCP client).
- `extensions/<name>/`: `hearth-extension.toml` plus an MCP server in any language.
- `fixtures/`: mock data, seeded-failure manifests, `clusters.toml`.

## Design rules

- **Host:** Rust stable, ratatui 0.30, crossterm, tokio. No other TUI framework.
- **Modes:** Triage, Agent, Receipts. Keep state across switches. A command palette reaches every action. Textual gave these free; we build them, so keep them small.
- **External TUIs:** k9s, lazygit, `$EDITOR`. Leave the alternate screen, run the child, restore.
- **Extensions:** each is a separate MCP server process launched by the host. No in-process plugins. The host is the MCP client (`rmcp`). No `hearth-extension.toml`, no launch. It declares Kubernetes verbs, resources, and namespaces; domains; read, write, and deny paths; and secret names. All else is denied.
- **Two layers:** the policy engine denies first. The mock cluster's service account has real RBAC, so a denied restart holds even if the sandbox fails. Show why.
- **Sandbox:** every extension and agent tool process runs through `SandboxBackend`. Linux: shell out to system `bwrap`, plus the `landlock` crate. Not the `bubblewrap` crate (0.0.1 placeholder). macOS: `/usr/bin/sandbox-exec` (Seatbelt; deprecated, still used by Codex CLI). Optional: Anthropic `srt` (needs Node, research preview). Deny reads of `~/.ssh`, `~/.kube`, `~/.docker`, keyrings, `.env`.
- **Network:** allow-only, from manifest domains, through a Hearth-owned egress proxy. The proxy injects secrets; the process never sees the key.
- **Broker:** Kubernetes (`kube`), Docker (`bollard`), and image builds go through the host broker only. Sandboxed processes get no kubeconfig and no Docker socket.
- **Approvals:** a person approves cluster writes, image pushes, writes outside scratch, and promoting a fix.
- **Receipts:** append-only JSONL at `.hearth/receipts/YYYY-MM-DD.jsonl`, each entry SHA-256 chained to the previous one. It detects tampering only if the head hash is anchored outside the ledger: on screen at run end and in a commit. Record actor, model id, tool, decision, redacted inputs, outputs, approver, checks.
- **Triage:** read-only, mock data. Verdict (product bug or deployment issue), confidence, evidence, next action.
- **Config and secrets:** `serde` + `toml` config. Secrets come from the `keyring` crate (service `hearth`) or env vars, never config files. `.env` is local dev only, gitignored.
- **Models:** `ModelProvider` trait. Gemini first (the company has Gemini Enterprise), then others.
- **Environment probe:** read-only. Report confidence. Flag a Docker socket mounted in a container.
- **Stretch (post-demo):** mirrord and Vite. Never set Vite `server.allowedHosts` to `true`. No mirrord `steal` without an HTTP filter and an approval.

## How to work

1. Read the code the task touches. List every place the change must reach.
2. Take the first option that fully works: skip it, reuse existing code, use std or an existing dependency. Only then write new code.
3. No single-use helpers. No speculative abstractions, options, or "for later" code.
4. Delete before you add. Finish every caller and test the change breaks.
5. Bug fix: reproduce first, then fix the root cause once, in the shared code.
6. Before you add complexity for an unlikely edge case, state the failure mode, likelihood, and cost. Wait for a yes.
7. If you can answer a question by running something, run it. Ask only when a wrong guess is costly or cannot be undone.
8. Verify any new crate, flag, API, or version against official docs. If you cannot, mark it **⚠️ VERIFY**. Never guess.

## Prove it works

- Every "done" needs evidence: test output, run output, or a screenshot. "It compiles" is not proof.
- A behavior change needs at least one small test where that is cheap. No TDD ceremony.
- Policy and sandbox changes need a deny-path test. No exceptions.
- If a check fails, fix the code. Do not weaken or delete the test.

## Checks, not prose

When a mistake repeats, encode the fix as high as it goes: types and crate boundaries first, then lint or CI, then a line in this file, then human review.

CI gates (add in M0, then make CI a required status check on `master`): the full check above, plus a deny-path test for every policy or sandbox change.

| Rule | Enforced by |
|---|---|
| Model API code only in `hearth-providers` | only that crate depends on model and HTTP crates; `cargo-deny` `wrappers` |
| Kubernetes and Docker clients only in `hearth-broker` | only that crate depends on `kube`, `bollard`; `cargo-deny` `wrappers` |
| No direct process spawn outside the sandbox | clippy `disallowed_methods` on `std::process::Command::new`; `#[allow]` only in `hearth-sandbox` and TUI suspend |
| Mock clusters only | broker refuses any kube context not listed in `fixtures/clusters.toml` |
| Policy or sandbox change has a deny-path test | CI fails if `hearth-policy` or `hearth-sandbox` changes without a `deny_path` test |
| No em dashes in docs | CI grep |
| Tracker updated with the change | human review |

Add a row for each new correction.

## Style

- Code: `cargo fmt`. Parse at boundaries (manifests, config, CLI, MCP, broker) into typed structs; trust them inside. No `unwrap` outside tests. Prefer early returns. Comment only the why. Mark a known shortcut `// shortcut: <limit>, <when to upgrade>`.
- Writing (docs, PRs, replies): loosely ASD-STE100. Lead with the answer. Short sentences. Active voice. Imperative steps. No em dashes. Colons are fine.

## Never

- Never commit secrets, kubeconfigs, `.env` files, or unredacted receipts.
- Never weaken a default deny. A new capability needs a manifest change, a deny-path test, and a receipt type.
- Never give a sandboxed process a raw kubeconfig or the Docker socket.
- Never add a dependency without asking.
- Never rewrite pushed history or push to `master` directly.
- Never cut validation at trust boundaries, security, or error handling that prevents data loss.

## Git and PRs

- Small diffs. One logical change per PR.
- PRs go into `master`. One approval is required. No force push.
- Branch: at most three words, hyphenated. Commit: `type(scope): summary`.
- PR body: start with a short plain-language summary anyone can read (what and why, no jargon). Then what changed, why, and the evidence. Use `.github/pull_request_template.md`. Update `TRACKER.md` in the same PR.

## Report back

- Lead with the result.
- List files changed and commands run, with output.
- Label each claim: measured, inferred, or guess.
- End with what you skipped or did not check, and any risk.

## Sources (checked 2026-10-23)

- ratatui 0.30.2: https://crates.io/crates/ratatui
- rmcp 3.7.0 (official Rust MCP SDK, implements MCP 2026-07-28): https://github.com/modelcontextprotocol/rust-sdk
- landlock 0.4.7, kube 4.2.0, bollard 0.21.1, keyring 4.2.0: crates.io
- Codex sandboxes: https://github.com/openai/codex/blob/main/codex-rs/linux-sandbox/README.md and https://github.com/openai/codex/blob/main/codex-rs/sandboxing/src/seatbelt.rs
- cargo-deny bans: https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html
- mirrord config: https://metalbear.com/mirrord/docs/config
- Vite server options: https://vite.dev/config/server-options
