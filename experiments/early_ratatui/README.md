# Early Ratatui experiments

First sketches of a Rust terminal UI with [Ratatui](https://ratatui.rs/) and
Crossterm, made while choosing Hearth's stack. They are kept for reference and are
not part of the Cargo workspace (the root `Cargo.toml` excludes `experiments/`).

| Path | What it is |
|---|---|
| [ratatui-simple.rs](ratatui-simple.rs) | Loose single-file sketch: a counter changed with the arrow keys |
| [ratui-demo-basic.rs](ratui-demo-basic.rs) | Loose single-file sketch: a selectable list |
| [ratui-demo/](ratui-demo/) | Standalone crate (ratatui 0.30, crossterm 0.29) for trying the sketches |

**Status: does not compile.** As of 2026-10-28, `cargo build` in `ratui-demo/`
fails with 12 errors against ratatui 0.30, and the loose `.rs` files have no crate
of their own. Treat them as notes, not working code. The real TUI belongs in
`crates/` (see [AGENTS.md](../../AGENTS.md)).
