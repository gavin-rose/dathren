pub mod brfss_mental_health;
pub mod cdc_injury_county;
pub mod cdc_places;
pub mod cdc_specific_drugs;
pub mod cdc_vsrr;
pub mod cdc_vsrr_county;
pub mod census_population;
pub mod mtf_youth_substance;
pub mod nchs_maternal;
pub mod nchs_mortality;
pub mod nhis_mental_health;
mod socrata;
pub mod world_bank_health;
pub mod yrbss_mental_health;

pub(crate) fn number(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse().ok())
}

pub(crate) fn month_number(name: &str) -> Option<u8> {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS.iter().position(|m| *m == name).map(|i| i as u8 + 1)
}
