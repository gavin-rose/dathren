# Dathren

Public health data, cleaned up and documented.

Public health data is scattered across agency portals, and each source has its
own quirks: provisional counts that get revised, categories that overlap,
suppressed cells that look like zeros. Dathren gives these datasets one clean
interface and keeps the caveats attached to the data.

- **`dathren`** (`crates/dathren-core`): Rust library. Fetches each source and normalizes it into
  `Observation`s with flags such as `suppressed`, `underreported`, and `provisional`.
- **`dathren-cli`**: the `dathren` command for listing, inspecting, and exporting datasets.
- **`dathren-site`**: generates the static catalog site, with one page per
  dataset and a discussion thread for each.
- **`dathren-py`**: the `dathren` Python package (PyO3).
- **`dathren-node`**: the `dathren` npm package for Node.js (napi-rs).

## Python and Node

```sh
pip install dathren        # or: pip install 'dathren[pandas]'
npm install dathren
```

```python
import dathren
df = dathren.fetch_df("nchs-mortality-rates", geo="MT", measure="suicide")
```

```js
const dathren = require("dathren");
const rows = await dathren.fetch("nchs-mortality-rates", { geo: "MT", measure: "suicide" });
```

Both return the same flat records as the CLI's CSV. To build them locally:

```sh
cd crates/dathren-py && maturin develop --release && pytest tests
cd crates/dathren-node && npm install && npm run build && npm test
```

## Usage

```sh
cargo install dathren-cli                # or: cargo add dathren
cargo run -p dathren-cli -- list
cargo run -p dathren-cli -- info cdc-vsrr-overdose
cargo run -p dathren-cli -- fetch cdc-vsrr-overdose --geo US --measure synthetic_opioids --since 2020
cargo run -p dathren-cli -- fetch cdc-vsrr-overdose --geo OH --format json -o ohio.json
```

## Datasets

| Id | Source | Geography |
|----|--------|-----------|
| `cdc-vsrr-overdose` | CDC NCHS provisional drug overdose death counts | US, states |
| `nchs-mortality-rates` | CDC NCHS provisional death rates by cause | US, states |
| `brfss-mental-health` | CDC BRFSS mental health indicators | States |
| `cdc-vsrr-county-overdose` | CDC NCHS provisional county-level drug overdose deaths | Counties |
| `cdc-places-county` | CDC PLACES local health estimates | Counties |

## Website

```sh
cargo run -p dathren-site           # reads site/, writes dist/
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

## Releasing

1. Bump `version` in `Cargo.toml` and `crates/dathren-node/package.json`.
2. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.

The `Release` workflow builds wheels and Node addons for Linux, macOS, and
Windows, tests them, and publishes to crates.io, PyPI, and npm.

npm releases are staged, not published directly. Approve them on npmjs.com
under **Staged Packages**, or with `npm stage approve <stage-id>`. Approve the
five platform packages (`@dathren/dathren-darwin-arm64` and so on) along with `dathren`.

## License

MIT
