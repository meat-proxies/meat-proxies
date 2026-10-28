# Alternative: Python/Textual TUI (superseded)

These files are from an earlier plan to build Hearth as a Python app on
[Textual](https://textual.textualize.io/). The project chose a Rust host with
Ratatui, Crossterm, and Tokio instead (see [AGENTS.md](../AGENTS.md) and
[the product plan](../docs/product-plan.md)). They are kept for reference only:
nothing builds, installs, or tests them.

| File | What it is |
|---|---|
| [pyproject.toml](pyproject.toml) | Package config for a Textual app (`hearth.app:main`), with textual, pydantic-settings, kubernetes, keyring, mirrord, and mcp dependencies |
| [final_structure.txt](final_structure.txt) | Planned Python package layout: Textual screens, workers, widgets, Pydantic schemas, pytest tests |
| [PROJECT_ANALYSIS.md](PROJECT_ANALYSIS.md) | Notes from that planning pass, including next steps for Textual scaffolding and `textual.pilot` tests |
| [AGENTS.md](AGENTS.md) | Sentences about the Python/Textual plan cut from the root `AGENTS.md` (a record, not agent instructions) |
| [product-plan.md](product-plan.md) | Sentences about the Python/Textual plan cut from `docs/product-plan.md` |

The analysis also mentions files that are not Python-specific and live elsewhere:
`hearth-extension.toml` at the repo root (the extension manifest example, any
language) and the early Ratatui demos, now in
[experiments/early_ratatui/](../experiments/early_ratatui/README.md).
