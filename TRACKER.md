# TRACKER.md

Requirements, milestones, and open items for Hearth. Update this file in the same PR as the change.

Status: ✅ done · 🚧 in progress · ⬜ not started · ❌ dropped. Baseline: 2026-10-23.

## Milestones (demo first)

| # | Milestone | Done when | Rows |
|---|---|---|---|
| M0 | Workspace, gates, types, TUI shell | Cargo workspace with the crates in `AGENTS.md`. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass in CI on every PR; CI is a required status check on `master`. Manifest, policy, and receipt types frozen in `hearth-core` (day 1). Three modes (Triage, Agent, Receipts), command palette, and suspend to an external TUI work (day 2). `hearth doctor` checks bwrap, user namespaces, and kind. A script creates a local kind cluster with seeded failures. | R1, R2, R3, R8, R19, R20 |
| M1 | Thin slice: the demo runs | One read-only Kubernetes extension as an MCP server process with a manifest. Policy engine denies the pod restart; cluster RBAC also denies it. bwrap sandbox plus egress proxy blocks the undeclared host. Fix tested in scratch, approval key, mock triage verdict, receipt with head hash on screen. Gemini provider. Recorded run of the 60-second demo. | R4, R11, R12, R13, R14, R15, R17, R18 |
| M2 | Widen | More extensions in other languages (HTTP, images via broker), setup wizard, environment probe, second provider adapter, macOS Seatbelt backend. | R5, R6, R7, R9, R16, R21 |
| M3 | Demo hardening | Demo rehearsed three times on demo hardware. Backup recording. Deny-path tests cover every demo beat. | R18 |
| Stretch | Post-demo | mirrord and Vue/Vite deploy into a mock cluster. | R10 |

## Requirements

| # | Requirement | Milestone | Status |
|---|---|---|---|
| R1 | Runs in a terminal (one Rust binary, `hearth`) | M0 | ⬜ |
| R2 | Displays TUI graphics (ratatui + crossterm) | M0 | ⬜ |
| R3 | Fast mode switching with state kept between switches; command palette | M0 | ⬜ |
| R4 | Extensible: any-language MCP server processes plus capability manifests | M1 | ⬜ |
| R5 | Uses standard tools (ratatui, tokio, serde, toml, keyring, kube, bollard, rmcp) | M2 | ⬜ |
| R6 | Safe handling of config, secrets, keys, and tokens; setup shows what is missing and where to put it | M2 | ⬜ |
| R7 | Short setup (install the binary, then `hearth setup` and `hearth doctor`) | M2 | ⬜ |
| R8 | Launches and returns from other TUI apps (leave alternate screen, run child, restore) | M0 | ⬜ |
| R9 | Reads local system, container status, network, kubeconfig, Docker socket access | M2 | ⬜ |
| R10 | Deploy a local Vue + Vite app into a mock cluster with mirrord | Stretch | ⬜ |
| R11 | Extensions declare what they can touch (default deny) | M1 | ⬜ |
| R12 | Per-run sandbox (bwrap + landlock; Seatbelt on macOS): no real keys, no undeclared network | M1 | ⬜ |
| R13 | Every change leaves a receipt; head hash anchored outside the ledger | M1 | ⬜ |
| R14 | Triage: product bug or deployment issue, on mock data | M1 | ⬜ |
| R15 | Human approval before any cluster-changing action | M1 | ⬜ |
| R16 | Model-agnostic through `hearth-providers` (Gemini first) | M2 | ⬜ |
| R17 | Host is the MCP client to extensions (revision 2026-07-28, `rmcp`) | M1 | ⬜ |
| R18 | 60-second demo runs end to end | M1, M3 | ⬜ |
| R19 | Mock data only: fixtures and a local kind cluster with real RBAC; broker refuses unlisted kube contexts | M0 | ⬜ |
| R20 | CI gates: fmt, clippy, test, deny-path test for policy and sandbox changes; crate boundaries enforced; required on `master` | M0 | ⬜ |
| R21 | Kubernetes, Docker, and image builds go through the host broker only | M2 | ⬜ |

## Changes log

Add a row for each new or changed requirement.

| Date | Row | Change | Requester |
|---|---|---|---|
| 2026-10-23 | R10 | Moved to post-demo stretch | Micah |
| 2026-10-23 | R19, R20, R21 | Added | Micah |
| 2026-10-23 | R1 to R5, R7, R8, R12, R16, R17, R19, R20 | Stack switched to Rust host with out-of-process MCP extensions. R3 and R8 moved to M0; R17 moved to M1. | Micah |

## Open items

- Stack: Rust host, any-language MCP extensions. Chosen 2026-10-23 after three independent reviews. Fallback: the same design ports to Python + Textual.
- Branch ruleset on `master` has a CodeQL code-scanning gate, but CodeQL default setup was not configured as of 2026-10-23. It may block merges until enabled. CodeQL supports Rust (GA, default setup, build mode `none`).
- Official demo length and judging rules: not yet published by organizers.
- Event OpenAI access: form unknown (API keys or seats).
- bwrap user-namespace availability on demo hardware. Some distros restrict unprivileged user namespaces. ⚠️ VERIFY with `hearth doctor` on the actual machine.
- Egress proxy secret injection (header injection for HTTPS without giving the process the key). ⚠️ VERIFY design and test in M1.
- macOS `sandbox-exec` is marked deprecated by Apple. Risk: removal or behavior change. Codex CLI still uses it. ⚠️ VERIFY on the demo Mac if we demo on macOS.
- Mode switch under 50 ms to first paint: unmeasured. ⚠️ VERIFY with a benchmark in M0.
- Gemini Rust client: no official Rust SDK confirmed. Plan: REST over HTTP in `hearth-providers`. ⚠️ VERIFY.
- mirrord `port_mapping` element order (`[local, remote]` assumed). ⚠️ VERIFY before R10.
