# Hearth / meat-proxies

One terminal where developers and agents use the same extension tools under the
same enforced rules. Built by team meat proxies for the hackathon.

The Rust workspace currently provides a help/version executable, an empty core
crate for issue #3, and a hidden bonus game ([Slop Runner](#bonus-slop-runner)).
The TUI, extensions, policy, sandbox, and demo are not implemented.

```sh
cargo run -p hearth -- --help
./scripts/check --quick
./scripts/check
```

Install the pinned Rust toolchain with rustup and Python 3.11 or later.
See [development and review](docs/development.md), [agent instructions](AGENTS.md),
[review priorities](REVIEW.md), and [the product plan](docs/product-plan.md).
[GitHub issues](https://github.com/meat-proxies/meat-proxies/issues) define acceptance;
[FEATURE_REQUESTS.md](FEATURE_REQUESTS.md) tracks which features are completed, in
progress, and in the backlog.

## Bonus: Slop Runner

A hidden endless runner, left out of `--help` on purpose. Run through a world
filling up with AI slop, rendered as 24-bit pixel art right in your terminal.

```sh
cargo run -p hearth -- --slop
```

| Key | Action |
|---|---|
| `space`, `↑`, `w`, `k`, `Enter` | Jump; on the game-over screen, run again |
| `↓`, `s`, `j` | Duck (hold to keep ducking); in the air, drop faster |
| `p` | Pause or resume |
| `q`, `Esc`, `Ctrl-C` | Quit |

Jump over ground slop, duck under the flying em-dash swarm, and grab gold sparks
for 50 points each. The run speeds up as you go, and the sky turns from afternoon
to sunset to night as your score climbs. The slop to watch for:

- **A puddle of lukewarm filler content**: wobbling beige goo
- **A six-fingered hand**: reaching up out of the ground
- **A tower of listicles**: the tallest thing you have to jump
- **An em-dash swarm**: flies at head height, so duck (shows up once you get going)

The runner wears Claude-style colors: a cream dress with a terracotta sash and
hem, terracotta ribbons in her twin tails, and charcoal tights and boots.

**Graphics.** It picks the best look your terminal can show:

| Terminal | What you get |
|---|---|
| Kitty, WezTerm, Ghostty (Kitty image protocol) or iTerm2 | High-detail art as real images, at any window size |
| Any other 24-bit color terminal, at least 128×47 | High-detail art drawn with half-block characters |
| Any other 24-bit color terminal, smaller (minimum 64×24) | The original small pixel art |

Inside tmux or screen it uses half blocks, since they hide which terminal is
outside. To force a mode, set `HEARTH_SLOP_GRAPHICS` to `kitty`, `iterm`,
`blocks`, or `auto` (the default), for example
`HEARTH_SLOP_GRAPHICS=blocks cargo run -p hearth -- --slop`. Image modes send
about 4 MB/s on an 80×24 window, so prefer `blocks` over slow SSH links.

The best score lasts until you quit and is not saved.

## Agent skills: Superpowers

This project uses [Superpowers](https://github.com/obra/superpowers), a library of
workflow skills for coding agents (brainstorming, writing plans, test-driven
development, systematic debugging, code review).

- **Claude Code:** the repo's `.claude/settings.json` registers the plugin, so
  Claude Code should offer to install it when you trust the folder. To install it
  by hand, run `/plugin marketplace add obra/superpowers-marketplace` and then
  `/plugin install superpowers@superpowers-marketplace` inside Claude Code. From a
  shell, the same steps are `claude plugin marketplace add obra/superpowers-marketplace`
  and `claude plugin install superpowers@superpowers-marketplace --scope project`.
  Check it with `claude plugin list`.
- **Codex CLI:** run `/plugins`, search for "superpowers", and choose Install Plugin.
- **OpenCode:** tell it "Fetch and follow instructions from
  https://raw.githubusercontent.com/obra/superpowers/refs/heads/main/.opencode/INSTALL.md".

Where a Superpowers skill conflicts with [AGENTS.md](AGENTS.md), AGENTS.md wins.

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
