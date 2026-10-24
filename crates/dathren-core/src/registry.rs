use crate::sources::cdc_vsrr::CdcVsrrOverdose;
use crate::{Error, Result, Source};

static SOURCES: &[&dyn Source] = &[&CdcVsrrOverdose];

pub fn all() -> &'static [&'static dyn Source] {
    SOURCES
}

pub fn get(id: &str) -> Result<&'static dyn Source> {
    SOURCES
        .iter()
        .copied()
        .find(|s| s.meta().id == id)
        .ok_or_else(|| Error::UnknownDataset(id.to_owned()))
}
