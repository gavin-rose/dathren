use crate::sources::brfss_mental_health::BrfssMentalHealth;
use crate::sources::cdc_injury_county::CdcInjuryCounty;
use crate::sources::cdc_places::CdcPlaces;
use crate::sources::cdc_specific_drugs::CdcSpecificDrugs;
use crate::sources::cdc_vsrr::CdcVsrrOverdose;
use crate::sources::cdc_vsrr_county::CdcVsrrCounty;
use crate::sources::census_population::CensusPopulation;
use crate::sources::mtf_youth_substance::MtfYouthSubstance;
use crate::sources::nchs_maternal::NchsMaternal;
use crate::sources::nchs_mortality::NchsMortality;
use crate::sources::nhis_mental_health::NhisMentalHealth;
use crate::sources::world_bank_health::WorldBankHealth;
use crate::sources::yrbss_mental_health::YrbssMentalHealth;
use crate::{Error, Result, Source};

static SOURCES: &[&dyn Source] = &[
    &CdcVsrrOverdose,
    &CdcSpecificDrugs,
    &NchsMortality,
    &NchsMaternal,
    &BrfssMentalHealth,
    &YrbssMentalHealth,
    &NhisMentalHealth,
    &MtfYouthSubstance,
    &CdcVsrrCounty,
    &CdcInjuryCounty,
    &CdcPlaces,
    &WorldBankHealth,
    &CensusPopulation,
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
