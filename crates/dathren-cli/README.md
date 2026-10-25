# dathren-cli

Public health data from the command line, cleaned up and documented.

```sh
cargo install dathren-cli

dathren list
dathren info cdc-vsrr-overdose
dathren fetch cdc-vsrr-overdose --geo US --measure synthetic_opioids --since 2024
dathren fetch brfss-mental-health --geo CA --format json -o california.json
```

`--geo` takes `US`, a state abbreviation, or a 5-digit county FIPS code. Output
is CSV by default, with suppressed values left blank and flagged rather than
written as zero.

Browse datasets and their caveats at [dathren.debugr.net](https://dathren.debugr.net).
