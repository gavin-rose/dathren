## What this changes

## Checklist

- [ ] `cargo fmt`, `cargo clippy --workspace --all-targets`, and `cargo test` pass
- [ ] New datasets have caveats, tests for row parsing, and are registered in `registry.rs`
- [ ] Suppressed or missing values stay `None` with a flag, never zero
