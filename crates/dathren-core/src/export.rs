use std::io::Write;

use crate::Observation;

pub fn write_csv<W: Write>(w: W, observations: &[Observation]) -> csv::Result<()> {
    let mut out = csv::Writer::from_writer(w);
    out.write_record([
        "dataset",
        "measure",
        "geo",
        "geo_name",
        "period",
        "value",
        "adjusted_value",
        "ci_low",
        "ci_high",
        "flags",
        "note",
    ])?;
    for o in observations {
        let flags = o
            .flags
            .iter()
            .map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join(";");
        let num = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
        out.write_record([
            o.dataset.as_str(),
            &o.measure,
            &o.geo,
            o.geo_name.as_deref().unwrap_or_default(),
            &o.period.to_string(),
            &num(o.value),
            &num(o.adjusted_value),
            &num(o.ci_low),
            &num(o.ci_high),
            &flags,
            o.note.as_deref().unwrap_or_default(),
        ])?;
    }
    out.flush()?;
    Ok(())
}
