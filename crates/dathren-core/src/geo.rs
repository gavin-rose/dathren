use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct State {
    pub abbr: &'static str,
    pub fips: &'static str,
    pub name: &'static str,
}

#[rustfmt::skip]
pub static STATES: &[State] = &[
    State { abbr: "AL", fips: "01", name: "Alabama" },
    State { abbr: "AK", fips: "02", name: "Alaska" },
    State { abbr: "AZ", fips: "04", name: "Arizona" },
    State { abbr: "AR", fips: "05", name: "Arkansas" },
    State { abbr: "CA", fips: "06", name: "California" },
    State { abbr: "CO", fips: "08", name: "Colorado" },
    State { abbr: "CT", fips: "09", name: "Connecticut" },
    State { abbr: "DE", fips: "10", name: "Delaware" },
    State { abbr: "DC", fips: "11", name: "District of Columbia" },
    State { abbr: "FL", fips: "12", name: "Florida" },
    State { abbr: "GA", fips: "13", name: "Georgia" },
    State { abbr: "HI", fips: "15", name: "Hawaii" },
    State { abbr: "ID", fips: "16", name: "Idaho" },
    State { abbr: "IL", fips: "17", name: "Illinois" },
    State { abbr: "IN", fips: "18", name: "Indiana" },
    State { abbr: "IA", fips: "19", name: "Iowa" },
    State { abbr: "KS", fips: "20", name: "Kansas" },
    State { abbr: "KY", fips: "21", name: "Kentucky" },
    State { abbr: "LA", fips: "22", name: "Louisiana" },
    State { abbr: "ME", fips: "23", name: "Maine" },
    State { abbr: "MD", fips: "24", name: "Maryland" },
    State { abbr: "MA", fips: "25", name: "Massachusetts" },
    State { abbr: "MI", fips: "26", name: "Michigan" },
    State { abbr: "MN", fips: "27", name: "Minnesota" },
    State { abbr: "MS", fips: "28", name: "Mississippi" },
    State { abbr: "MO", fips: "29", name: "Missouri" },
    State { abbr: "MT", fips: "30", name: "Montana" },
    State { abbr: "NE", fips: "31", name: "Nebraska" },
    State { abbr: "NV", fips: "32", name: "Nevada" },
    State { abbr: "NH", fips: "33", name: "New Hampshire" },
    State { abbr: "NJ", fips: "34", name: "New Jersey" },
    State { abbr: "NM", fips: "35", name: "New Mexico" },
    State { abbr: "NY", fips: "36", name: "New York" },
    State { abbr: "NC", fips: "37", name: "North Carolina" },
    State { abbr: "ND", fips: "38", name: "North Dakota" },
    State { abbr: "OH", fips: "39", name: "Ohio" },
    State { abbr: "OK", fips: "40", name: "Oklahoma" },
    State { abbr: "OR", fips: "41", name: "Oregon" },
    State { abbr: "PA", fips: "42", name: "Pennsylvania" },
    State { abbr: "RI", fips: "44", name: "Rhode Island" },
    State { abbr: "SC", fips: "45", name: "South Carolina" },
    State { abbr: "SD", fips: "46", name: "South Dakota" },
    State { abbr: "TN", fips: "47", name: "Tennessee" },
    State { abbr: "TX", fips: "48", name: "Texas" },
    State { abbr: "UT", fips: "49", name: "Utah" },
    State { abbr: "VT", fips: "50", name: "Vermont" },
    State { abbr: "VA", fips: "51", name: "Virginia" },
    State { abbr: "WA", fips: "53", name: "Washington" },
    State { abbr: "WV", fips: "54", name: "West Virginia" },
    State { abbr: "WI", fips: "55", name: "Wisconsin" },
    State { abbr: "WY", fips: "56", name: "Wyoming" },
    State { abbr: "GU", fips: "66", name: "Guam" },
    State { abbr: "PR", fips: "72", name: "Puerto Rico" },
    State { abbr: "VI", fips: "78", name: "U.S. Virgin Islands" },
];

pub fn state(abbr: &str) -> Option<&'static State> {
    STATES.iter().find(|s| s.abbr.eq_ignore_ascii_case(abbr))
}

/// Matches names written as `New York` or `new_york`.
pub fn state_by_name(name: &str) -> Option<&'static State> {
    let wanted = name.replace('_', " ");
    STATES.iter().find(|s| s.name.eq_ignore_ascii_case(&wanted))
}

/// Pads county FIPS codes that upstream stores as numbers, e.g. `1001` to `01001`.
pub fn county_fips(raw: &str) -> String {
    format!("{:0>5}", raw.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_states() {
        assert_eq!(state("ny").unwrap().fips, "36");
        assert_eq!(state_by_name("district_of_columbia").unwrap().abbr, "DC");
        assert_eq!(county_fips("1001"), "01001");
    }
}
