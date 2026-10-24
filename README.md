# Dathren

Public health data, cleaned up and documented.

Public health data is scattered across agency portals, and each source has its
own quirks: provisional counts that get revised, categories that overlap,
suppressed cells that look like zeros. Dathren gives these datasets one clean
interface and keeps the caveats attached to the data.

- **`dathren-core`**: Rust library. Fetches each source and normalizes it into
  `Observation`s with flags such as `suppressed`, `underreported`, and `provisional`.
- **`dathren-cli`**: the `dathren` command for listing, inspecting, and exporting datasets.
- **`dathren-site`**: generates the static catalog site, with one page per
  dataset and a discussion thread for each.

## Usage

```sh
cargo run -p dathren-cli -- list
cargo run -p dathren-cli -- info cdc-vsrr-overdose
cargo run -p dathren-cli -- fetch cdc-vsrr-overdose --geo US --measure synthetic_opioids --since 2020
cargo run -p dathren-cli -- fetch cdc-vsrr-overdose --geo OH --format json -o ohio.json
```

## Datasets

| Id | Source |
|----|--------|
| `cdc-vsrr-overdose` | CDC NCHS provisional drug overdose death counts |

## Website

```sh
cargo run -p dathren-cli -- site    # reads site/, writes dist/
python3 -m http.server -d dist      # preview at http://localhost:8000
```

The `Site` workflow builds and deploys to GitHub Pages on every push to `main`
and once a day, so the site picks up new upstream data.

To set it up:

1. In the repo, go to **Settings → Pages** and set the source to **GitHub Actions**.
2. Under **Settings → Pages → Custom domain**, enter `dathren.debugr.net`. DNS
   (on Squarespace) has a `CNAME` record for `dathren` pointing at `gavin-rose.github.io`.
3. For discussion threads, enable **Discussions** on the repo, install
   [giscus](https://giscus.app), and fill in `[giscus]` in `site/config.toml`.

## Adding a dataset

1. Add a module under `crates/dathren-core/src/sources/` that implements `Source`.
2. Write the dataset's `caveats`. These are as important as the code.
3. Register it in `registry.rs`.

## License

MIT
