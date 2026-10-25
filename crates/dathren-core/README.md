# dathren

Public health data, cleaned up and documented.

dathren fetches datasets from agencies like the CDC and returns them in one
consistent shape, with the quirks marked instead of hidden. Suppressed cells
come back as `None` with a `Suppressed` flag, never as zero. Provisional counts
are flagged, and the publisher's own adjusted values sit next to the reported
ones.

```rust,no_run
use dathren::{Client, Query, registry};

fn main() -> dathren::Result<()> {
    let source = registry::get("cdc-vsrr-overdose")?;
    let query = Query {
        geo: Some("US".into()),
        measures: vec!["synthetic_opioids".into()],
        since_year: Some(2024),
    };
    for o in source.fetch(&Client::new()?, &query)? {
        println!("{} {:?} {:?}", o.period, o.best(), o.flags);
    }
    Ok(())
}
```

Every dataset documents its measures and the caveats to read before drawing
conclusions:

```rust
for source in dathren::registry::all() {
    let meta = source.meta();
    println!("{}: {} caveats", meta.id, meta.caveats.len());
}
```

Browse datasets, charts, and caveats at
[dathren.debugr.net](https://dathren.debugr.net). Also available as the
`dathren-cli` command-line tool and as `dathren` on PyPI and npm.
