# AGENTS.md

Hearth is one terminal for developers and their agents. Every tool is an extension. Agents use the same tools, under the same rules, as people. Python 3.12+, Textual, uv. "Hearth" is a working name.

We build it for the Beast Code internal hackathon on Fri Oct 30 2026. Pitch: `docs/pitch.md`. Requirements and status: `TRACKER.md`.

## Hard rules

1. **No Beast Code data or code.** Event rule: "No Beast Code data or code - use a mock data set if you need to." Every cluster, log, incident, and image is a mock fixture: files in `fixtures/` or a local kind or k3d cluster with seeded failures. No real kubeconfigs, tokens, hostnames, or registry URLs. Do not name or recreate internal tools. A company touchpoint is a stub we define.
2. **Enforce in code and the OS, never in a prompt.**
3. **Default deny.** An extension touches only what its manifest declares.
4. **Agents never hold real secrets.** The SecretBroker injects them at the tool boundary.
5. **Every change leaves a receipt.**
6. **Model-agnostic.** Any provider plugs in through `hearth/providers/`. No provider SDK import outside that folder. No personal billing or account rules in this repo.

## The demo is the acceptance test

We ask the agent to fix a broken mock service. It triages and finds a deployment issue. It tries to restart a pod it was never granted: the shell stops it. It tries to call an undeclared host: the shell blocks it. It tests its fix in the sandbox, a person approves, the check passes, and the receipt is on screen. 60 seconds.

If a change does not move this demo or a `TRACKER.md` row, stop and ask.

**Build order:** first a thin slice that runs the demo: one mode, one extension, one policy deny, one receipt, mock triage. Then widen. mirrord and Vue/Vite deploy are post-demo stretch. See `TRACKER.md`.

## Commands

The repo has no code yet. M0 adds these. Keep them stable.

- Install: `uv sync`
- Run: `uv run hearth`
- Lint: `uv run ruff check`
- Format: `uv run ruff format`
- Test all: `uv run pytest`
- Test one: `uv run pytest tests/test_x.py::test_name`
- Full check before done: `uv run ruff check && uv run pytest`

## Layout (target)

- `hearth/`: `app.py` (Textual app), `policy/`, `broker/`, `sandbox/`, `receipts/`, `providers/`.
- `extensions/<name>/`: one package each, with `hearth-extension.toml`.
- `fixtures/`: mock data and seeded-failure manifests.

## Design rules

- **Stack:** Python + Textual. It replaced an earlier Rust/ratatui idea; pending Chris's agreement. Do not add another TUI framework to the core.
- **Modes:** one Textual mode per tool. Install long-lived screens. Pause polling on `ScreenSuspend`. Run I/O in workers.
- **External TUIs:** launch them inside `App.suspend()`. Detect with `shutil.which`.
- **Extensions:** entry-point group `hearth.extensions` plus a required `hearth-extension.toml`. No manifest, no load. It declares RBAC verbs and resources, domains, paths, and secret names. Each tool is exposed to the TUI and as an MCP tool.
- **Two layers:** PolicyEngine denies first. Kubernetes RBAC still enforces server-side. Show why a call was denied.
- **Sandbox:** every agent tool process runs through `SandboxBackend` (default `srt`, a research preview). Network is allow-only, from the manifests. Deny reads of `~/.ssh`, `~/.kube`, `~/.docker`, keyring stores, and `.env`. Unix sockets are blocked.
- **Broker:** Kubernetes calls, Docker calls, and image builds all go through the Hearth broker. The sandbox blocks Unix sockets, including the Docker socket, so a sandboxed process cannot reach a daemon or API server directly. The broker checks policy and writes a receipt.
- **Approvals:** a person approves cluster writes, image pushes, writes outside scratch, and promoting a fix.
- **Receipts:** append-only JSONL at `.hearth/receipts/YYYY-MM-DD.jsonl`, each entry SHA-256 chained to the previous one. The chain detects tampering only if the head hash is anchored outside the ledger: show it on screen at the end of a run and record it in a commit. Otherwise anyone who can write the file can rewrite the whole chain. Record actor, model id, tool, policy decision, redacted inputs, outputs, approver, and check results.
- **Triage:** read-only, mock data only. Output a verdict (product bug or deployment issue), confidence, evidence, and a next action.
- **Config and secrets:** `pydantic-settings` with prefix `HEARTH_`. Secret fields are `SecretStr`. Secrets live in `keyring` (service `hearth`), never in config files. `.env` is for local dev only and is gitignored.
- **Environment probe:** read-only. Report confidence, not certainty. Flag a Docker socket mounted into a container.
- **Stretch (post-demo):** mirrord and Vite. Never set Vite `server.allowedHosts` to `true`. No mirrord `steal` without an HTTP filter and an approval.

## How to work

1. Read the code the task touches. List every place the change must reach.
2. Take the first option that fully works: skip it, reuse existing code, use the standard library or an installed dependency. Only then write new code.
3. No single-use helpers. No speculative abstractions, options, or "for later" code.
4. Delete before you add. Finish every caller and test the change breaks.
5. Bug fix: reproduce first, then fix the root cause once, in the shared code.
6. Before you add complexity for an unlikely edge case, state the failure mode, likelihood, and cost. Wait for a yes.
7. If you can answer a question by running something, run it. Ask only when a wrong guess is costly or cannot be undone.
8. Verify any new library, flag, API, or version against official docs. If you cannot, mark it **⚠️ VERIFY**. Never guess.

## Prove it works

- Every "done" needs evidence: test output, run output, or a screenshot. "It compiles" is not proof.
- A behavior change needs at least one small test where that is cheap. No TDD ceremony.
- Policy and sandbox changes need a deny-path test. No exceptions.
- If a check fails, fix the code. Do not weaken or delete the test.

## Checks, not prose

When a mistake repeats, encode the fix as high as it goes: codebase or types first, then lint or CI, then a line in this file, then human review.

CI gates (add in M0, then make CI a required status check on `master`): `uv run ruff check`, `uv run pytest`, and a deny-path check for every policy or sandbox change.

| Rule | Enforced by |
|---|---|
| Provider SDKs only in `hearth/providers/` | ruff `TID251` banned-api, with a per-file exemption |
| Kubernetes and Docker clients only in `hearth/broker/` | ruff `TID251` banned-api, with a per-file exemption |
| Mock clusters only | broker refuses any kube context not listed in `fixtures/clusters.toml` |
| Policy or sandbox change has a deny-path test | CI fails if `hearth/policy/` or `hearth/sandbox/` changes without a test marked `deny_path` |
| No em dashes in docs | CI grep |
| Tracker updated with the change | human review |

Add a row each time you encode a correction.

## Style

- Code: `ruff format`. Type hints on public functions. Validate at boundaries (manifests, config, CLI, broker responses) and trust typed values inside. Prefer early returns. Comment only the why. Mark a known shortcut `# shortcut: <limit>, <when to upgrade>`.
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

- textual 8.2.8 (PyPI, 2026-06-30): https://pypi.org/project/textual/
- keyring 25.7.0: https://pypi.org/project/keyring/
- MCP 2026-07-28 (stable; no `initialize` handshake, no protocol sessions): https://modelcontextprotocol.io/specification/2026-07-28/changelog
- Sandbox Runtime: https://github.com/anthropic-experimental/sandbox-runtime
- ruff banned-api: https://docs.astral.sh/ruff/rules/banned-api/
- mirrord config: https://metalbear.com/mirrord/docs/config
- Vite server options: https://vite.dev/config/server-options
