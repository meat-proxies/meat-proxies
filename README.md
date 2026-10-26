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
