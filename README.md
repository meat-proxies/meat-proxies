# Hearth / meat-proxies

One terminal where developers and agents use the same extension tools under the
same enforced rules. Built by team meat proxies for the hackathon.

The Rust workspace currently provides a help/version executable and an empty core
crate for issue #3. The TUI, extensions, policy, sandbox, and demo are not implemented.

```sh
cargo run -p hearth -- --help
./scripts/check --quick
./scripts/check
```

Install the pinned Rust toolchain with rustup and Python 3.11 or later.
See [development and review](docs/development.md), [agent instructions](AGENTS.md),
[review priorities](REVIEW.md), and [the product plan](docs/product-plan.md).
[GitHub issues](https://github.com/meat-proxies/meat-proxies/issues) define acceptance
and track progress.
Feature status lives in [FEATURE_REQUESTS.md](FEATURE_REQUESTS.md).

## Hosting on Entire

This project has been ported to [Entire](https://entire.com), a Git forge built
for teams working with AI coding agents. Entire hosts the Git repository and,
through its CLI, records agent sessions (prompts, transcripts, tool calls, and
file changes) and links them to commits as *checkpoints*. You can browse code,
commits, and sessions together on entire.com.

### Install the CLI

```sh
# Linux (Ubuntu 24.04 / Arch)
curl -fsSL https://entire.com/install.sh | bash

# macOS
brew install --cask entireio/tap/entire

# Verify; both binaries must be on PATH
entire version
command -v git-remote-entire
```

The installer adds `entire` and `git-remote-entire`, a Git remote helper that lets
plain `git` talk to `entire://` URLs.

### Log in and get the code

```sh
entire login
entire repo clone /et/<project>/<repo>
```

Repository remotes use the form `entire://<host>/et/<project>/<repo>`, where
`<host>` is the regional cluster the repository lives in (for example
`aws-us-east-2.entire.io`). Run `entire repo view /et/<project>/<repo>` to see the
exact remote. To switch an existing checkout over:

```sh
git remote set-url origin entire://<host>/et/<project>/<repo>
```

After that, `git pull` and `git push` work as usual.

### Capture agent sessions

Run this once in your checkout:

```sh
entire enable
```

It installs Git hooks and hooks for supported agents (Claude Code, Gemini CLI,
OpenCode, Cursor, and others). Keep working with your agent as normal. Each commit
gets a checkpoint, stored on a separate `entire/checkpoints/v1` branch, so your
working branch stays clean. Checkpoints let reviewers see why a change was made,
and the CLI can rewind to or resume from earlier sessions.

Secret redaction in captured sessions is best-effort. Hearth's rules still apply:
never paste credentials, kubeconfigs, or unredacted receipts into an agent
session. See the [Entire docs](https://docs.entire.com) for the full CLI reference.
