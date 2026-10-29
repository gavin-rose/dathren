# dathren-cli

Public health data from the command line, cleaned up and documented.

```sh
cargo install dathren-cli

dathren list
dathren info cdc-vsrr-overdose
dathren fetch cdc-vsrr-overdose --geo US --measure synthetic_opioids --since 2024
dathren fetch brfss-mental-health --geo CA --format json -o california.json
```

`--geo` takes `US`, a state abbreviation, a 5-digit county FIPS code, or a
3-letter country code. Output is CSV by default, with suppressed values left
blank and flagged rather than written as zero.

```sh
# Each county's newest value, as deaths per 100,000 residents
dathren fetch cdc-vsrr-county-overdose --geo OH --latest --per-capita

# Sum counties without treating suppressed cells as zero
dathren total cdc-vsrr-county-overdose --geo OH --since 2026 --latest
# 2180 to 2540 (2135 reported from 43 values, plus 45 suppressed)

dathren cite cdc-vsrr-overdose --style bibtex
```

Browse datasets and their caveats at [dathren.debugr.net](https://dathren.debugr.net).
