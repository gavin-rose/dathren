use std::io::Write;

use crate::Observation;

pub fn write_csv<W: Write>(w: W, observations: &[Observation]) -> csv::Result<()> {
    let mut out = csv::Writer::from_writer(w);
    out.write_record([
        "dataset",
        "measure",
        "geo",
        "period",
        "value",
        "adjusted_value",
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
        out.write_record([
            o.dataset.as_str(),
            &o.measure,
            &o.geo,
            &o.period.to_string(),
            &o.value.map(|v| v.to_string()).unwrap_or_default(),
            &o.adjusted_value.map(|v| v.to_string()).unwrap_or_default(),
            &flags,
            o.note.as_deref().unwrap_or_default(),
        ])?;
    }
    out.flush()?;
    Ok(())
}
