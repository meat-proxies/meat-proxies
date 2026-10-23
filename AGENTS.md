# AGENTS.md — Overarching Goal & Agent Guide

> **Working name:** `Hearth` (placeholder — rename freely; the command-line entry point is referred to as `hearth` below).
> **Format:** This file follows the open **AGENTS.md** convention — plain Markdown, no required schema, a "README for agents." Nested `AGENTS.md` files in sub-folders (e.g. `extensions/k8s/AGENTS.md`) may add package-specific rules; the nearest file to the code being edited wins.
> **Last verified against sources:** 2026-10-23. Every version number, API name and config key below was checked against the official source listed in [§14 Sources](#14-sources). Anything not yet verified is explicitly marked **⚠️ VERIFY**.

---

## 1. Mission (read this first)

**We are building one beautiful terminal for developers and their agents.**

Every tool is an **extension**: your app's pods and volumes, a quick HTTP call, image builds, and **triage** that tells you in seconds whether a failure is a **product bug** or a **deployment issue**. Agents work in the **same shell, with the same tools, under the same rules** as humans.

Non-negotiable principles:

1. **Each extension declares what it can touch.** No declaration → no access.
2. **Every agent run gets its own sandbox** — no real keys, and no network it didn't ask for.
3. **Every change leaves a receipt.** Nothing touches production on trust alone.
4. **Sandboxed by the shell, not by the prompt.** Enforcement lives in code/OS (Operating System) primitives, never in instructions to a model.
5. **Model-agnostic.** Works with any model provider (Gemini, Grok, Claude, or whatever comes next) behind one adapter interface.

### 1.1 The 60-second demo (this is the acceptance test for v1)

> We ask the agent to fix a broken service. It **triages** and finds a **deployment issue**. It tries to **restart a pod it was never granted** — the shell stops it. It tries to **call a host it never declared** — the shell blocks that too. It **tests its fix in the sandbox**, a **person approves**, the **check passes**, and the **proof (receipt) is on screen**. That took 60 seconds, and nothing touched production on trust alone.

Agents: if a change you are making does not move us toward this demo or one of the requirements in §11, stop and ask.

### 1.2 Why it wins (the four "awards")

| Award | What it means for the product |
|---|---|
| 🪄 **Magic Wand** | Triage + fixes; five tools (kubectl, docker, curl/HTTP client, log viewer, image builder) collapsed into one shell. Saves hours per engineer per week. |
| 🧪 **Elixir of Life** | Engineers stop babysitting agents and stop dreading the 2 AM "is it us or the deploy?" page. Teams can try new agents without fear. |
| 🧙 **Sorcerer's Apprentice** | Any team (platform, QA — Quality Assurance, security, support) ships an extension. **Their tools and their rules travel together** into everyone's terminal. |
| 🚫 **Banishment** | Banishes tab sprawl, copy-pasted `kubectl` commands, and "trust me, the agent said it's fixed." |

---

## 2. Framework decision: Python **Textual** (on Rich) vs Rust **Ratatui**

### ✅ Decision: **Python + Textual** (Textual is the TUI framework; Rich is the rendering library it builds on)

| Requirement | Textual (Python) — verified | Ratatui (Rust) — verified |
|---|---|---|
| Runs in terminal, TUI (Text User Interface) graphics | Yes. Latest **textual 8.2.8**, released 2026-06-30, requires Python ≥ 3.9, MIT licence, "Production/Stable". | Yes. Latest **ratatui 0.30.x** (0.30.2 highlights published). |
| **Very fast page/tool switching with state saved between switches** | **Built in.** *Modes* are named screen stacks (`MODES`, `App.switch_mode`, `DEFAULT_MODE`); switching modes shows the top screen of that stack. *Installed screens* (via `SCREENS` / `install_screen`) are kept in memory for the app's lifetime. `ScreenSuspend` / `ScreenResume` events fire on switch so hidden pages can pause polling. | **Not built in.** Ratatui uses *immediate-mode rendering* (UI redrawn every frame from app state); the developer owns the render loop, event loop, and app architecture ("little to no help in organizing large applications"). |
| Use other TUI apps in the ecosystem (k9s, lazygit, vim, …) | **Built in:** `App.suspend()` context manager temporarily leaves application mode to run another terminal program, then restores the app (official example launches vim). Note: not available under textual-web. | Possible (ratatui has a "Spawn External Editor (Vim)" recipe) but hand-wired. |
| Command palette / discoverability | **Built in** fuzzy command palette on `Ctrl+P`, extensible with custom commands. | Build yourself. |
| Extensibility (third-party extensions) | Python **entry points** (`[project.entry-points.'<group>']` + `importlib.metadata.entry_points(group=...)`) — the PyPA (Python Packaging Authority) recommended plugin-discovery mechanism. Extensions ship as normal `pip`/`uv` packages. | Rust has no stable runtime plugin ABI (Application Binary Interface); extensions would need to be compiled in or run as subprocesses. |
| Standard, popular tooling for this domain | Official Kubernetes Python client (in-cluster + kubeconfig loaders), `keyring`, `pydantic-settings`, `uv`. | Rust crates exist but the team/extension authors (platform, QA, security, support) are more likely to write Python. |
| Testing | Built-in testing framework (`textual.pilot`) + `textual-dev` dev console. | Snapshot testing via `insta` recipe. |
| Bonus | `textual serve` can serve the same app in a browser. | — |

**Trade-off accepted:** Rust/Ratatui would give a single static binary and lower memory. We accept a Python runtime in exchange for built-in screen/mode state retention, built-in suspend-to-external-TUI, a command palette, and a far lower barrier for extension authors. Install friction is mitigated with `uv tool install` (isolated environment, executables placed on `PATH`).

**Rule for agents:** do **not** introduce Ratatui, curses, prompt_toolkit or another TUI framework into the core. Rust is allowed only for an optional, separately-packaged helper binary if a measured performance problem demands it (record an ADR — Architecture Decision Record — first).

---

## 3. Target stack (all verified to exist; pin versions in `pyproject.toml`)

| Concern | Choice | Notes |
|---|---|---|
| Language / runtime | Python ≥ 3.12 (project choice; Textual itself supports ≥ 3.9) | |
| Packaging & install | `uv` — `uvx hearth` (run without installing) or `uv tool install hearth` (installs into an isolated venv — virtual environment — and puts executables on `PATH`) | |
| TUI | `textual` 8.2.x (+ `textual-dev` for development) | |
| Config | `pydantic-settings` (`BaseSettings`) — sources: init args, environment variables, `.env` (dotenv) files, secrets directory, plus `pyproject.toml`/custom sources | Env vars override dotenv; dotenv/env override secrets dir. |
| Secrets at rest | `keyring` 25.7.x → macOS Keychain, Windows Credential Locker, Freedesktop Secret Service (GNOME), KWallet (KDE) | See §6 caveats. |
| Kubernetes | Official `kubernetes` Python client: kubeconfig loader + in-cluster loader | |
| Containers / images | Docker Engine API via the user's Docker context / `DOCKER_HOST`; image builds via BuildKit/`docker buildx` | |
| Local ↔ cluster dev loop | **mirrord** (MetalBear) | §8 |
| Agent sandbox | **Anthropic Sandbox Runtime (`srt`, npm `@anthropic-ai/sandbox-runtime`)** — OS-level sandbox: `bubblewrap` on Linux, `sandbox-exec` (Seatbelt) on macOS, proxy-based network allowlist | **Research preview**, config format "provisional" — wrap behind our own `SandboxBackend` interface so it can be swapped. |
| Agent ↔ tool protocol | **MCP (Model Context Protocol)**, current spec revision **2026-07-28** (stateless core; no session handshake) | Every extension tool is exposed both to the TUI and as MCP tools so any model/agent can call them. |
| Model providers | Internal `ModelProvider` adapter interface (Gemini, Grok/xAI, Claude, OpenAI, local) | No provider SDK imported outside `hearth/providers/`. |

---

## 4. Architecture (high level)

```
┌──────────────────────────── hearth (Textual App) ────────────────────────────┐
│ Modes (one screen stack each, state retained):                               │
│  [Home/Context] [Pods & Volumes] [HTTP] [Images] [Triage] [Deploy+mirrord]   │
│  [Agent] [Receipts] [Setup]  …+ any extension-provided mode                  │
│  Ctrl+P command palette  •  App.suspend() → k9s / lazygit / vim / $EDITOR    │
├───────────────────────────────────────────────────────────────────────────────┤
│ Core services                                                                 │
│  EnvironmentProbe  ConfigService  SecretBroker  PolicyEngine  ReceiptLedger   │
│  ExtensionLoader (entry points)   MCPToolBus     SandboxBackend (srt)         │
│  ModelProvider adapters                                                       │
├───────────────────────────────────────────────────────────────────────────────┤
│ Extensions (each = Python package + manifest of declared capabilities)       │
│  k8s • http • images • triage • deploy-mirrord • <team extensions>            │
└───────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Fast switching with retained state
- Each tool is a **mode** registered in `App.MODES`; switching uses `App.switch_mode(name)` bound to number keys and the palette.
- Long-lived screens are **installed** so they persist for the app's lifetime; never recreate a screen on switch.
- On `ScreenSuspend`, pause timers/watches (e.g. Kubernetes watch streams); on `ScreenResume`, resume and diff — do not reload from scratch.
- Heavy I/O (Input/Output) runs in Textual **workers** so the UI thread never blocks.
- Target: mode switch perceived as instant (< 50 ms to first paint) — **⚠️ VERIFY** with a benchmark in CI (Continuous Integration) once the skeleton exists.

### 4.2 Using other TUI apps
- An `ExternalTool` registry maps names → commands (`k9s`, `lazygit`, `vim`, `$EDITOR`, …). Availability is detected at runtime (`shutil.which`), never assumed.
- Launch inside `with self.suspend():` then return to the exact mode/screen the user left.
- External tools launched **by an agent** go through the same PolicyEngine and sandbox as any other command.

---

## 5. Extension model ("Sorcerer's Apprentice")

An extension = **code + manifest**. Tools and rules travel together.

- **Discovery:** Python entry-point group `hearth.extensions` (declared in the extension's `pyproject.toml`).
- **Manifest (`hearth-extension.toml`)** — required; the loader refuses extensions without one:

```toml
[extension]
name = "k8s-pods"
version = "0.1.0"
owner = "platform-team"

[capabilities.kubernetes]
# Mapped 1:1 onto Kubernetes RBAC (Role-Based Access Control) verbs/resources.
allow = [
  { verbs = ["get", "list", "watch"], resources = ["pods", "pods/log", "persistentvolumeclaims"], namespaces = ["dev", "staging"] },
]
# NOT declared → "delete pods" / rollout restart is impossible for this extension and for any agent using it.

[capabilities.network]
allowed_domains = ["api.github.com"]   # empty list = no network

[capabilities.filesystem]
read  = ["."]
write = ["./.hearth/tmp"]
deny_read = ["~/.ssh", "~/.kube", ".env"]

[capabilities.secrets]
# Names only. Values are never given to the extension/agent; the SecretBroker injects them at the boundary.
uses = ["GITHUB_TOKEN"]

[mcp]
expose_tools = true
```

- **Two-layer enforcement:**
  1. **Hearth PolicyEngine** — every tool call is checked against the union of manifest capabilities for the active extension(s) *before* execution.
  2. **Real authority** — Kubernetes still enforces RBAC server-side (access is denied by default; a denied request returns HTTP 403 Forbidden). Hearth pre-checks with `kubectl auth can-i …` / the `SelfSubjectAccessReview` API and shows the user *why* something is blocked.
- Policy decisions (allow/deny + reason) are written to the receipt ledger.

---

## 6. Configuration, secrets, API keys & tokens

### 6.1 Rules
- **Config ≠ secrets.** Config lives in `~/.config/hearth/config.toml` (user) and `./.hearth/config.toml` (project). Secrets never go in either.
- Settings model = `pydantic-settings` `BaseSettings`, env prefix `HEARTH_`. Mark every secret field `SecretStr` so it is redacted in logs/repr.
- **Priority:** CLI (Command-Line Interface) flags > environment variables > `.env` > secrets directory (`/run/secrets` in containers, Docker secrets convention) > keyring > defaults. (Env-over-dotenv-over-secrets ordering is pydantic-settings' documented default; keyring is our custom source.)
- **Secret storage on a workstation:** `keyring` (OS keychain). Service name `hearth`, username = secret name.
- **Caveats to surface in the UI (verified from keyring docs):**
  - macOS: any Python script run by the same Python executable can read secrets created by keyring without an OS prompt. Setup wizard explains how to require a prompt via Keychain Access → Access Control.
  - Headless Linux / containers: Secret Service needs a D-Bus session + unlocked GNOME Keyring; if unavailable, fall back to the secrets directory or env vars and **say so explicitly**.
- `.env` files are allowed for local dev only; setup adds `.env` and `.hearth/secrets*` to `.gitignore`.
- Agents **never** receive raw secret values. The SecretBroker substitutes placeholders and injects real credentials only at the egress proxy / tool boundary (design — **⚠️ VERIFY** feasibility of header injection through the `srt` proxy; fallback is a Hearth-owned forward proxy).

### 6.2 Guided setup (`hearth setup` and the **Setup** mode)
Concise, idempotent, re-runnable. It walks the user through *only what is missing*:

1. **Probe** environment (§7) and show a checklist: ✅ found / ❌ missing / ⚠️ degraded.
2. For each missing item, show **exactly what to copy and where** (e.g. "Paste your Gemini API key — stored in macOS Keychain as `hearth/GEMINI_API_KEY`").
3. Validate each secret with a cheap, read-only call before saving.
4. Never echo a secret back; show last 4 characters only.
5. Write a setup receipt.

---

## 7. Environment awareness (EnvironmentProbe)

Runs at startup and on demand; results feed the **Home/Context** mode and the agent's context.

| Signal | How (verified mechanism) |
|---|---|
| **Inside a container?** | `/.dockerenv` exists (Docker); `/run/.containerenv` exists (Podman); fallbacks: `/proc/self/mountinfo` overlay paths, `/proc/self/cgroup` containing `docker`/`kubepods`/`containerd`. Note: markers can be absent (e.g. BuildKit builds, `/run` volume mounts) or spoofed — report confidence, not certainty. |
| **Inside a Kubernetes pod?** | `KUBERNETES_SERVICE_HOST` + `KUBERNETES_SERVICE_PORT` env vars, and service-account files at `/var/run/secrets/kubernetes.io/serviceaccount/{token,ca.crt,namespace}`. |
| **Kubeconfig** | `--kubeconfig` flag (single file, no merge) > `KUBECONFIG` env var (list of files, `:`-separated on Linux/macOS, `;` on Windows, merged) > `$HOME/.kube/config`. Show current context, cluster, namespace, user. |
| **Kubernetes permissions** | `kubectl auth can-i` / `SelfSubjectAccessReview` for each capability an extension declares; `kubectl auth whoami` for identity. |
| **Docker socket / daemon** | Order: `--host` / `--context` flags > `DOCKER_HOST` / `DOCKER_CONTEXT` env vars > active `docker context` (metadata under `~/.docker/contexts/`) > default `unix:///var/run/docker.sock`. Docker Desktop (macOS 4.13+ and Linux) may not create `/var/run/docker.sock`; read the endpoint from `docker context inspect`. Also report whether the socket is **mounted into this container** (high-risk: root-equivalent) and flag it. |
| **Network** | Interfaces + addresses, default route, DNS (Domain Name System) resolvers, proxy env vars (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`), reachability of the Kubernetes API server. Library choice (e.g. `psutil`) **⚠️ VERIFY** before adding. |
| **Local system** | OS, architecture, shell, terminal size/colour support, presence of `kubectl`, `docker`, `mirrord`, `k9s`, `lazygit`, `node`/`npm`, `srt`, `bwrap`. |

The probe is **read-only** and runs unsandboxed only for local file/env reads; any network probe obeys the network policy.

---

## 8. Deploy to cluster + mirrord (Vue + Vite local dev loop)

Goal: run a local web app (e.g. Vue + Vite dev server) **in the context of a pod** — receive the pod's incoming traffic, reach cluster services, read its env vars — without rebuilding images.

### 8.1 Verified mirrord facts
- mirrord runs a local process "in the context" of a target (pod / deployment / Argo Rollout in OSS — Open Source Software edition) by creating an **agent pod** in the cluster that accesses the target pod's namespaces.
- Incoming traffic modes (`feature.network.incoming`): **`mirror`** (default; local gets a copy, local responses dropped) or **`steal`** (local process answers instead of the pod). Steal can be narrowed with `http_filter.header_filter` / `path_filter`.
- Outgoing traffic is sent from the target so the local app can reach cluster endpoints; `feature.network.outgoing.filter.local` keeps chosen hosts/ports local.
- Config file: `-f <path>` on the CLI, or `.mirrord/mirrord.json` (IDE — Integrated Development Environment — plugins). Supports Tera templating, `{{ key }}` session key, `{{ git_branch }}`.
- Relevant top-level keys: `target.path`, `target.namespace`, `kubeconfig`, `kube_context`, `agent.*`, `feature.env`, `feature.fs`, `feature.network.incoming.{mode,http_filter,port_mapping,ignore_ports}`.
- OSS limitation: with a multi-pod workload, mirrord targets **one random pod**. Stealing **HTTPS (HTTP Secure) traffic that the app itself TLS-terminates (Transport Layer Security)** with a filter requires mirrord for Teams.
- Without the mirrord Operator, if the target pod restarts you must restart the local mirrord session.

### 8.2 Verified Vite facts
- Dev server defaults: `server.host = 'localhost'`, `server.port = 5173`; if the port is taken Vite silently tries the next one unless `server.strictPort = true`.
- `server.allowedHosts` defaults to localhost/IPs; setting it to `true` exposes the dev server to DNS-rebinding attacks — **never** set `true`; add explicit hostnames only.
- `server.fs.deny` blocks `.env`, `.env.*`, cert/key files by default — keep it.
- HMR (Hot Module Replacement) runs over a WebSocket (`server.ws`); behind a proxy it may need `server.ws.clientPort`.

### 8.3 Feature flow (Deploy + mirrord mode)
1. **Detect** app type (Vite config present) and pick the port; force `strictPort: true` so the port mirrord maps is the port Vite actually binds.
2. **(Optional) Deploy:** build image (BuildKit/`docker buildx`) → push → apply manifests (Server-Side Apply) to a dev namespace. Every step is a policy-checked tool call with a receipt.
3. **Generate** `.mirrord/mirrord.json`, e.g.:

```json
{
  "target": { "path": "deployment/frontend", "namespace": "dev" },
  "kube_context": "dev-cluster",
  "feature": {
    "env": true,
    "fs": "read",
    "network": {
      "incoming": {
        "mode": "steal",
        "http_filter": { "header_filter": "^baggage: .*mirrord-session={{ key }}.*$" },
        "port_mapping": [[5173, 80]]
      },
      "outgoing": true
    }
  }
}
```
   **⚠️ VERIFY** the element order of `port_mapping` (`[local, remote]` assumed) against the mirrord config reference before shipping.
4. **Run** `mirrord exec -f .mirrord/mirrord.json -- npm run dev` inside a managed subprocess; stream logs into the mode; show the active session, target pod, and filter.
5. **Guardrails:** refuse `steal` without an HTTP filter on namespaces tagged `prod`; require human approval for any `steal`; receipt records target, filter, duration.

---

## 9. Agents: sandbox, approvals, receipts

### 9.1 Per-run sandbox
- Every agent run spawns its tool processes through `SandboxBackend` (default: `srt`):
  - **Network:** allow-only; `allowedDomains` = union of the active extensions' declared domains. Empty list = no network. Everything else is blocked by the proxy.
  - **Filesystem:** deny-read for `~/.ssh`, `~/.kube`, `~/.docker`, keyring stores, `.env`; write only to the run's scratch dir + declared paths.
  - **Unix sockets** (including the Docker socket) blocked by default.
- Kubernetes calls go through Hearth's broker (not a raw kubeconfig inside the sandbox) so the PolicyEngine sees every verb.
- **Platform caveat:** `srt` is a research preview; Linux needs `bubblewrap` (+ possibly `socat`, `ripgrep`); domain wildcards like `*.example.com` do not match the apex `example.com`. Check these in setup.

### 9.2 Human approval gates
Required before: any write/delete/patch to a cluster, any `steal`, any image push, any change outside the sandbox scratch dir, and promotion of a sandbox-tested fix.

### 9.3 Receipts ("every change leaves a receipt")
- Append-only JSONL (JSON Lines) ledger at `./.hearth/receipts/YYYY-MM-DD.jsonl`, each entry hash-chained (SHA-256 — Secure Hash Algorithm 256-bit — of previous entry) so tampering is detectable. (Design choice; no external standard claimed.)
- Each receipt: who (human/agent + model id), what tool, declared capability used, policy decision, inputs (secrets redacted), outputs/diff, sandbox id, approver, check results, timestamps.
- The **Receipts** mode renders them; the demo ends on a receipt.

### 9.4 Triage ("product bug or deployment issue?")
Collect in parallel (read-only): pod status/events, recent rollout history, image digest diff vs last good, container restarts/OOMKilled (Out Of Memory), config/secret change times, app error logs. Classify:
- **Deployment issue** signals: rollout in progress/failed, image/config changed near incident start, CrashLoopBackOff, probe failures, scheduling/resource errors.
- **Product bug** signals: stable rollout, same image as last good, application-level exceptions/5xx correlated with specific routes.
Output: verdict + confidence + evidence links + suggested next action (which may require approval).

---

## 10. Working rules for coding agents in this repo

1. **Verify before you assert.** Any new library, flag, API, or version must be checked against official docs; add the link to §14. If you cannot verify, mark **⚠️ VERIFY** — never guess.
2. Run `uv run pytest` and `uv run ruff check` before proposing a change. (Exact commands finalised once the skeleton exists — **⚠️ VERIFY**.)
3. Never commit secrets, kubeconfigs, `.env` files, or receipts containing unredacted data.
4. UI changes need a Textual pilot test; policy changes need a test proving the deny path.
5. Do not weaken a default-deny. New capabilities require a manifest change + test + receipt entry type.
6. Keep provider-specific code inside `hearth/providers/`; keep sandbox-specific code inside `hearth/sandbox/`.
7. Update the tracker in §11 (emoji status) in the same PR (Pull Request) as the change.

---

## 11. Requirements tracker

**Legend:** ✅ implemented · 🚧 in progress · ⬜ not started · 🆕 newly added · 🔄 changed · ❌ dropped

### 11.1 Core requirements (baseline 2026-10-23)

| # | Requirement | Status |
|---|---|---|
| R1 | Runs in a terminal | ⬜ |
| R2 | Displays TUI graphics (Textual) | ⬜ |
| R3 | Very fast page/tool switching in one session with state saved between switches (modes + installed screens) | ⬜ |
| R4 | Extensible (entry-point extensions + capability manifests) | ⬜ |
| R5 | Uses standardized, industry-popular tools (Textual, uv, pydantic-settings, keyring, Kubernetes client, Docker, mirrord, MCP) | ⬜ |
| R6 | Best-practice handling of configs/secrets/API keys/tokens; walks user through what's missing / what to copy in | ⬜ |
| R7 | Concise, simple setup (`uvx hearth` → `hearth setup`) | ⬜ |
| R8 | Can launch/return from other TUI apps (`App.suspend`) | ⬜ |
| R9 | Understands local system, container status, network, kubeconfig, Docker socket access | ⬜ |
| R10 | Deploy into cluster + mirrord a local web app (Vue + Vite) into a pod | ⬜ |
| R11 | Extensions declare what they can touch (default deny) | ⬜ |
| R12 | Per-agent-run sandbox: no real keys, no undeclared network | ⬜ |
| R13 | Every change leaves a receipt (hash-chained ledger + Receipts mode) | ⬜ |
| R14 | Triage: product bug vs deployment issue in seconds | ⬜ |
| R15 | Human approval gate before any production-affecting change | ⬜ |
| R16 | Model-agnostic (Gemini, Grok, others via adapters) | ⬜ |
| R17 | Tools exposed to agents via MCP (spec 2026-07-28) | ⬜ |
| R18 | 60-second demo (§1.1) runs end-to-end | ⬜ |

### 11.2 Updated / new requirements

> Add new rows here. Use 🆕 for new, 🔄 for changed (link the original R#), then move status through ⬜ → 🚧 → ✅. Include date and requester.

| # | Date | Requirement | Type | Status | Notes |
|---|---|---|---|---|---|
| N1 | YYYY-MM-DD | _example: support Podman socket in addition to Docker_ | 🆕 | ⬜ | |

---

## 12. Milestones

1. **M0 Skeleton** — Textual app, modes framework, command palette, Setup mode, EnvironmentProbe (R1–R3, R7, R9).
2. **M1 Extensions & policy** — loader, manifests, PolicyEngine, receipts (R4, R11, R13).
3. **M2 Core extensions** — k8s pods/volumes, HTTP, images, external TUI launcher (R5, R8).
4. **M3 Agents** — provider adapters, MCP bus, `srt` sandbox, approvals (R12, R15–R17).
5. **M4 Triage + Deploy/mirrord** (R10, R14).
6. **M5 Demo hardening** (R18).

---

## 13. Glossary (all acronyms used)

| Term | Meaning |
|---|---|
| ABI | Application Binary Interface |
| ADR | Architecture Decision Record |
| API | Application Programming Interface |
| CI | Continuous Integration |
| CLI | Command-Line Interface |
| D-Bus | Desktop Bus (Linux inter-process messaging system) |
| DNS | Domain Name System |
| HMR | Hot Module Replacement (Vite live-reload of changed modules) |
| HTTP / HTTPS | HyperText Transfer Protocol / HTTP Secure |
| IDE | Integrated Development Environment |
| I/O | Input/Output |
| JSON / JSONL | JavaScript Object Notation / JSON Lines (one JSON object per line) |
| MCP | Model Context Protocol |
| OOM | Out Of Memory |
| OS | Operating System |
| OSS | Open Source Software (here: mirrord's free edition) |
| PR | Pull Request |
| PyPA | Python Packaging Authority |
| QA | Quality Assurance |
| RBAC | Role-Based Access Control (Kubernetes authorization mode) |
| SHA-256 | Secure Hash Algorithm, 256-bit |
| SOCKS5 | Socket Secure proxy protocol, version 5 |
| SSR | Server-Side Rendering |
| TCP | Transmission Control Protocol |
| TLS | Transport Layer Security |
| TOML | Tom's Obvious, Minimal Language (config format) |
| TUI | Text User Interface |
| UI | User Interface |
| venv | Python virtual environment |

---

## 14. Sources

- Textual (PyPI, version/requirements): https://pypi.org/project/textual/
- Textual Screens & Modes: https://textual.textualize.io/guide/screens/
- Textual App basics / `App.suspend`: https://textual.textualize.io/guide/app/
- Ratatui rendering model: https://ratatui.rs/concepts/rendering/
- AGENTS.md format: https://github.com/davidstackio/agents.md (mirror of the spec repo) · https://infoq.com/news/2025/08/agents-md/
- Python plugin discovery (entry points): https://packaging.python.org/en/latest/guides/creating-and-discovering-plugins/
- uv tools: https://docs.astral.sh/uv/concepts/tools/
- pydantic-settings: https://docs.pydantic.dev/latest/concepts/pydantic_settings/
- keyring: https://pypi.org/project/keyring/
- Kubeconfig loading/merging: https://kubernetes.io/docs/concepts/configuration/organize-cluster-access-kubeconfig/
- Kubernetes authorization: https://kubernetes.io/docs/reference/access-authn-authz/authorization/
- Accessing the API from a pod (service-account paths): https://v1-34.docs.kubernetes.io/docs/tasks/run-application/access-api-from-pod
- Kubernetes Python client in-cluster loader: https://github.com/kubernetes-client/python-base/blob/a41c44715241552de73361673152f3f0d0bb9bc4/config/incluster_config.py
- Docker contexts / `DOCKER_HOST`: https://docs.docker.com/engine/manage-resources/contexts/ · https://github.com/docker/for-mac/issues/6529 · https://github.com/docker/docs/issues/23308
- Container detection markers: https://docs.rs/isdocker · https://docs.rs/detect-container
- mirrord config reference: https://metalbear.com/mirrord/docs/config
- mirrord incoming traffic: https://metalbear.com/mirrord/docs/using-mirrord/incoming-traffic
- mirrord HTTPS stealing (Teams): https://metalbear.com/mirrord/docs/using-mirrord/incoming-traffic/steal-https
- mirrord FAQ (targets, agent pod): https://metalbear.com/mirrord/docs/faq
- mirrord with SSR frontends: https://metalbear.com/blog/frontend-ssr/
- Vite server options: https://vite.dev/config/server-options
- Anthropic Sandbox Runtime: https://github.com/anthropic-experimental/sandbox-runtime
- MCP spec 2026-07-28 release: https://github.com/modelcontextprotocol/specification/releases/tag/2026-07-28
