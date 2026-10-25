pub mod brfss_mental_health;
pub mod cdc_places;
pub mod cdc_vsrr;
pub mod cdc_vsrr_county;
pub mod nchs_mortality;
mod socrata;

pub(crate) fn number(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse().ok())
}
