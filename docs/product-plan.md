# Hearth product plan

Updated 2026-10-26 for the owner-approved Rust direction and current roadmap. This preserves the original product intent; it does not claim implementation. [AGENTS.md](../AGENTS.md) contains daily engineering instructions. [GitHub issues](https://github.com/meat-proxies/meat-proxies/issues) track acceptance and progress.

## Mission and demo

One beautiful terminal for developers and their agents: pods and volumes, HTTP, logs, image builds, and triage share discoverable commands, enforced permissions, and visible receipts. Any model provider works through a common adapter.

The target 60-second demo:

1. Triage a seeded broken service as a deployment issue, showing evidence.
2. Block an undeclared pod action and explain why.
3. Block access to an undeclared host through real sandbox/proxy enforcement.
4. Test a concrete fix in isolation or a disposable copy of the mock cluster.
5. Show passing checks and get human approval before promotion.
6. Finish on the receipt showing action, checks, approver, and chain head.

Timing is a target to measure. Keep a keyboard-only path, seed/reset commands, a rehearsed script, and a backup recording. Use synthetic data and explicitly allowlisted local kind clusters only.

The pitch connects triage and consolidated tools to **Magic Wand**, less agent babysitting to **Elixir of Life**, portable extension tools/rules to **Sorcerer's Apprentice**, and the removal of tab sprawl and unverified fixes to **Banishment**. Avoid unsupported time-saved claims.

## Architecture

**Rust host: Ratatui, Crossterm, Tokio.** The host owns retained mode state, rendering/event loop, command palette, and terminal handoff. Python/Textual APIs and Python entry-point discovery from the earlier plan are superseded.

```text
Keyboard / agent pane
        |
Shared tool dispatcher (MCP client)
        |
Policy + approval + receipt lifecycle
        |
Sandboxed extension MCP process
        |
Policy-checked host broker
        |
Kubernetes / Docker / credential-bearing egress
```

This describes trust boundaries, not a finalized wire protocol. Broker requests must carry an authenticated originating extension/run identity and pass policy; a process cannot choose a more privileged identity. Extensions may use any language but run out of process. Process separation alone is not isolation.

Planned services: environment probe, config, secret broker, extension supervisor, policy engine, receipt ledger, MCP dispatcher, sandbox backend, model adapters. Select and verify SDKs, MCP revision, and extension discovery when implementing them; old version claims are not verified compatibility guarantees.

Issue #9 suggests system `bwrap` on Linux and `sandbox-exec` on macOS behind `SandboxBackend`, with allowlisted egress through a proxy. This supersedes the old `srt` default. Verify backend support and restrictions on actual machines. Alternatives must preserve the interface and enforcement; unavailable isolation prevents execution.

## Capabilities, secrets, and receipts

`hearth-extension.toml` declares extension identity/owner, Kubernetes verbs/resources/namespaces, network domains, filesystem reads/writes/denials, and secret names. Freeze the schema with fixtures in #3; earlier example TOML is not a finalized protocol.

Effective authority is limited by the calling extension's manifest, run restrictions, cluster allowlist, and real backend permissions. Unrelated extensions do not widen it. Denials take precedence. Approval authorizes a concrete permitted action, never a missing capability. Kubernetes RBAC remains an independent server-side check; preflight checks do not grant authority.

No kubeconfig, Docker socket, raw keys, or credential-store access inside sandboxes. Broker coverage includes indirect mutations such as restart and mirrord agent creation. Credentials are injected only at authenticated tool/egress boundaries restricted to the approved destination. Test injection and redaction; do not assume a CONNECT proxy can inject HTTPS headers.

User/project config paths remain `~/.config/hearth/config.toml` and `./.hearth/config.toml`, with `HEARTH_` environment names. Define and test their merge rules. Planned applicable override ordering: CLI > environment > local-development dotenv > secrets directory > OS keyring > defaults. Secret values never belong in config. Choose Rust libraries and verify custom precedence; Python library behavior no longer applies.

Setup is concise, idempotent, and asks only for missing items. State storage location, validate with a policy-permitted read-only call, never echo secrets, and write a redacted receipt. Explain headless credential-store fallback and verify platform access/prompt behavior before promising it. Keep local secrets out of Git.

Planned ledger: `.hearth/receipts/YYYY-MM-DD.jsonl`. Finalize canonical serialization, first-entry/day-boundary rules, concurrent appends, and verification in #3/#8. Record actor/model, tool, extension/capability, policy reason, redacted inputs/results/diff, run/sandbox, approver, checks, timestamps, and chain links. Include denials and failures. Show head hash and broken-entry location. A local chain detects edits relative to a trusted head; it is not proof against a complete ledger rewrite. Ledger failure must not silently permit unaudited privileged execution.

Approval precedes cluster mutations, image push, mirrord steal, writes beyond run scratch, and promotion. Refusal/cancellation runs nothing and leaves a receipt. Bind approval and passing checks to the exact fix promoted; changed inputs need new authorization/checks.

## Environment, UI, and triage

The read-only environment probe feeds Home/Context and safe agent context: OS, architecture, shell, terminal, tool availability, container/pod indicators, kube context, Docker endpoint, interfaces/routes/DNS, and redacted proxy settings. Report confidence for container detection. Credential-bearing reads stay in the trusted host. Never automatically target an ambient cluster. Verify client flag/environment/context precedence before shipping. Flag mounted Docker sockets as privileged access. Network probes obey policy and use bounded, cancellable I/O.

`hearth doctor` checks sandbox prerequisites, Linux user namespaces, kind, and model-key presence, explaining each failure. A real isolation smoke test is stronger than executable discovery. Missing a model key disables assistance, not keyboard operation or deterministic triage.

Planned modes: Home/Context, Pods & Volumes, HTTP, Images, Triage, Deploy/mirrord, Agent, Receipts, Setup. Ship in issue order. Retain state on switch, pause hidden polling, keep I/O off rendering, and restore the terminal around external applications. The earlier <50 ms mode-switch target remains unmeasured; define measurement before reporting results.

Triage gathers read-only status/events, rollout history, image/config changes, restarts/OOM evidence, and logs. Deployment signals include bad/recent rollout, image/probe/config/scheduling/resource failures. Product signals include application exceptions or route-correlated failures with stable deployment evidence. Correlate signals rather than treating one as proof. Return verdict, confidence, evidence, and next action; expose insufficient evidence. Test seeded bad image tag, CrashLoop, and missing config without a model. Product-bug fixtures can strengthen the distinction.

## Stretch integrations

HTTP declares reachable hosts in its own manifest. Image builds use the broker without exposing Docker sockets; pushes require approval. Both use the shared gate and receipt lifecycle.

For mirrord/Vite, detect explicit ports, enforce strict port binding, preserve Vite file-deny defaults, and allow explicit hosts only; never `allowedHosts = true`. Verify current mirrord keys, port mapping order, mirror/steal semantics, filters, outgoing routing, target/restart limitations, and HMR WebSocket behavior before shipping. Any steal needs approval; production-tagged namespaces also reject unfiltered steal, although the current demo allows only local fixture clusters. Cover cluster-side agent creation through the broker.

The extension authoring guide/template must let a new author run hello-world through the real gate, sandbox, and receipts, including a demonstrated denial.

## Original requirement traceability

These preserve original R IDs, not a second completion tracker. Current milestones/acceptance live in GitHub.

| IDs | Goal | Related issues |
|---|---|---|
| R1–R3 | Terminal graphics and retained, fast mode switching | #4, #16 |
| R4, R11 | Extensions with default-deny manifests | #3, #6, #7, #20 |
| R5 | Standard tools/protocols, updated for Rust | #2, #6, #18, #19 |
| R6–R7 | Safe config/secrets and guided setup | #15 readiness; full setup remains broader scope |
| R8 | Launch/return from external TUIs | #4 |
| R9 | System/container/network/cluster/Docker awareness | #5, #15; full probe remains broader scope |
| R10 | mirrord/Vite cluster development | #19, stretch |
| R12 | Isolated runs, no keys or undeclared network | #9 |
| R13 | Hash-chained receipts and viewer | #3, #8 |
| R14 | Evidence-based bug/deployment triage | #10 |
| R15 | Human approval and checked promotion | #12, #13 |
| R16–R17 | Model adapters and shared MCP tools | #6, #11 |
| R18 | Repeatable demo | #14, #17 |

## Primary references and open checks

Check chosen versions at implementation time; source inclusion does not establish compatibility or prove enforcement.

- [Ratatui rendering](https://ratatui.rs/concepts/rendering/), [Crossterm](https://docs.rs/crossterm/latest/crossterm/), [Tokio](https://tokio.rs/tokio/tutorial)
- [Cargo tests](https://doc.rust-lang.org/cargo/commands/cargo-test.html), [Clippy](https://doc.rust-lang.org/clippy/usage.html), [rustfmt](https://github.com/rust-lang/rustfmt)
- [MCP specification](https://modelcontextprotocol.io/specification/), [Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Bubblewrap](https://github.com/containers/bubblewrap), [optional sandbox runtime](https://github.com/anthropic-experimental/sandbox-runtime)
- [Kubernetes authorization](https://kubernetes.io/docs/reference/access-authn-authz/authorization/), [kubeconfig](https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/)
- [kind](https://kind.sigs.k8s.io/), [Docker contexts](https://docs.docker.com/engine/manage-resources/contexts/)
- [mirrord config](https://metalbear.com/mirrord/docs/config), [incoming traffic](https://metalbear.com/mirrord/docs/using-mirrord/incoming-traffic), [Vite server options](https://vite.dev/config/server-options)

Before claiming readiness, prove backend availability on both platforms, proxy bypass prevention, secret injection/redaction, MCP compatibility, and mirrord/HMR behavior. [Documentation PR #21](https://github.com/meat-proxies/meat-proxies/pull/21) establishes this guidance and supersedes the closed [PR #1](https://github.com/meat-proxies/meat-proxies/pull/1); do not restore its competing tracker or obsolete implementation claims.
