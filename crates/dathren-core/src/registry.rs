use crate::sources::brfss_mental_health::BrfssMentalHealth;
use crate::sources::cdc_places::CdcPlaces;
use crate::sources::cdc_vsrr::CdcVsrrOverdose;
use crate::sources::cdc_vsrr_county::CdcVsrrCounty;
use crate::sources::nchs_mortality::NchsMortality;
use crate::{Error, Result, Source};

static SOURCES: &[&dyn Source] = &[
    &CdcVsrrOverdose,
    &NchsMortality,
    &BrfssMentalHealth,
    &CdcVsrrCounty,
    &CdcPlaces,
];

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
