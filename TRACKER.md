# TRACKER.md

Requirements, milestones, and open items for Hearth. Update this file in the same PR as the change.

Status: ✅ done · 🚧 in progress · ⬜ not started · ❌ dropped. Baseline: 2026-10-23.

## Milestones (demo first)

| # | Milestone | Done when | Rows |
|---|---|---|---|
| M0 | Skeleton and gates | `uv sync`, `uv run hearth`, `uv run ruff check`, `uv run pytest` work. CI runs ruff and pytest on every PR and is a required status check on `master`. A script creates a local kind or k3d cluster with seeded failures. | R1, R2, R19, R20 |
| M1 | Thin slice: the demo runs | One mode, one read-only k8s extension, PolicyEngine denies the pod restart, sandbox blocks the undeclared host, fix tested in scratch, approval key, mock triage verdict, receipt with head hash on screen. Recorded run of the 60-second demo. | R4, R11, R12, R13, R14, R15, R18 |
| M2 | Widen | More modes and extensions (HTTP, images via broker), state kept across mode switches, command palette, setup wizard, environment probe, external TUI launch, two or more provider adapters, MCP tool bus. | R3, R5 to R9, R16, R17, R21 |
| M3 | Demo hardening | Demo rehearsed three times. Backup recording. Deny-path tests cover every demo beat. | R18 |
| Stretch | Post-demo | mirrord and Vue/Vite deploy into a mock cluster. | R10 |

## Requirements

| # | Requirement | Milestone | Status |
|---|---|---|---|
| R1 | Runs in a terminal | M0 | ⬜ |
| R2 | Displays TUI graphics (Textual) | M0 | ⬜ |
| R3 | Fast mode switching with state kept between switches | M2 | ⬜ |
| R4 | Extensible: entry-point extensions plus capability manifests | M1 | ⬜ |
| R5 | Uses standard tools (Textual, uv, pydantic-settings, keyring, Kubernetes client, Docker, MCP) | M2 | ⬜ |
| R6 | Safe handling of config, secrets, keys, and tokens; setup shows what is missing and where to put it | M2 | ⬜ |
| R7 | Short setup (`uvx hearth`, then `hearth setup`) | M2 | ⬜ |
| R8 | Launches and returns from other TUI apps (`App.suspend`) | M2 | ⬜ |
| R9 | Reads local system, container status, network, kubeconfig, Docker socket access | M2 | ⬜ |
| R10 | Deploy a local Vue + Vite app into a mock cluster with mirrord | Stretch | ⬜ |
| R11 | Extensions declare what they can touch (default deny) | M1 | ⬜ |
| R12 | Per-run agent sandbox: no real keys, no undeclared network | M1 | ⬜ |
| R13 | Every change leaves a receipt; head hash anchored outside the ledger | M1 | ⬜ |
| R14 | Triage: product bug or deployment issue, on mock data | M1 | ⬜ |
| R15 | Human approval before any cluster-changing action | M1 | ⬜ |
| R16 | Model-agnostic through `hearth/providers/` | M2 | ⬜ |
| R17 | Tools exposed to agents over MCP (revision 2026-07-28) | M2 | ⬜ |
| R18 | 60-second demo runs end to end | M1, M3 | ⬜ |
| R19 | Mock data only: fixtures and a local kind or k3d cluster; broker refuses unlisted kube contexts | M0 | ⬜ |
| R20 | CI gates: ruff, pytest, deny-path check for policy and sandbox changes; required on `master` | M0 | ⬜ |
| R21 | Docker calls and image builds go through the Hearth broker | M2 | ⬜ |

R19 to R21 are new on 2026-10-23 (AGENTS.md review). R10 moved to stretch. R5 no longer lists mirrord.

## Changes log

Add a row for each new or changed requirement.

| Date | Row | Change | Requester |
|---|---|---|---|
| 2026-10-23 | R10 | Moved to post-demo stretch | Micah |
| 2026-10-23 | R19, R20, R21 | Added | Micah |

## Open items

- Stack: Python + Textual replaced an earlier Rust/ratatui idea. Needs Chris's agreement.
- Branch ruleset on `master` has a CodeQL code-scanning gate, but CodeQL default setup was not configured as of 2026-10-23. It may block merges until enabled.
- Official demo length and judging rules: not yet published by organizers.
- Event OpenAI access: form unknown (API keys or seats).
- Can the `srt` proxy inject secret headers? ⚠️ VERIFY. Fallback: a Hearth-owned forward proxy.
- mirrord `port_mapping` element order (`[local, remote]` assumed). ⚠️ VERIFY before R10.
- Mode switch under 50 ms to first paint: unmeasured. Add a benchmark once M2 lands.
