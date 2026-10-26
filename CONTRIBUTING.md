# Contributing to dathren

Thanks for helping. The most useful contributions are new datasets, better
caveats, and bug reports about data that looks wrong.

Not sure where to start? Ask in
[Discussions](https://github.com/gavin-rose/dathren/discussions) or pick an
issue labeled `good first issue`.

## Setup

You need Rust (stable). Python and Node are only needed for the bindings.

```sh
git clone https://github.com/gavin-rose/dathren && cd dathren
cargo test                                # core, CLI, and site
cargo run -p dathren-cli -- list          # try the CLI
cargo run -p dathren-site                 # build the site into dist/
python3 -m http.server -d dist            # preview at http://localhost:8000
```

## Adding a dataset

A dataset is one file in `crates/dathren-core/src/sources/`.
[`cdc_vsrr_county.rs`](crates/dathren-core/src/sources/cdc_vsrr_county.rs) is a
short example to copy.

1. **Describe it.** Fill in a `DatasetMeta`: id, title, publisher, source URL,
   update cadence, geographies, and measures with their units and codes.
2. **Write the caveats.** These matter as much as the code. Cover anything that
   could mislead someone: overlapping categories, suppressed or provisional
   values, rolling periods, changes in how a question was asked. Write them for
   a reader who has never seen the dataset.
3. **Fetch and normalize.** Implement `Source::fetch`. Return one `Observation`
   per value, and never turn a missing value into zero: set `value` to `None`
   and add `Flag::Suppressed`. Data on Socrata portals like data.cdc.gov can use
   the helper in `sources/socrata.rs`.
4. **Test the tricky parts.** Add unit tests for row parsing: suppression,
   period formats, geography codes. Tests must not need the network.
5. **Register it** in `crates/dathren-core/src/registry.rs`.
6. **Check it end to end:**
   ```sh
   cargo run -p dathren-cli -- info your-dataset-id
   cargo run -p dathren-cli -- fetch your-dataset-id --geo US
   cargo run -p dathren-site && python3 -m http.server -d dist
   ```

Prefer publishers with a stable public API and a license that allows
redistribution. If a code system or dataset can't be redistributed (CPT, SNOMED
CT), link to it instead.

## Before opening a pull request

```sh
cargo fmt
cargo clippy --workspace --all-targets
cargo test
```

Keep comments for things the code can't say on its own, like why an upstream
quirk is handled a certain way.

## Reporting data problems

If a value looks wrong, open an issue with the dataset id, measure, geography,
and period, plus a link to where the publisher shows something different. Quirks
that aren't bugs but should be documented belong in the
[Data gotchas](https://github.com/gavin-rose/dathren/discussions) discussion
category.
