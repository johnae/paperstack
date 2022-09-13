# A toy transaction engine written in Rust

This is a toy transaction engine written in Rust. It can be used like this:

```sh
cargo run -r -- sampledata/transactions.csv
```

Unit tests can be run like this:

```sh
cargo test
```

It's been tested with larger data sets and performs ok for some definition of ok I guess :-). I'm sure this could be further optimized in multiple ways. What was guiding me at this point was mainly to use as few dependencies as possible and to keep it reasonably simple.

It uses the [Decimal crate](https://crates.io/crates/rust_decimal) as this engine makes financial calculations and f64 and friends can result in round-off errors.

It also uses [anyhow](https://crates.io/crates/anyhow) for easy error handling which seemed appropriate in this context. Other than that the [csv crate](https://crates.io/crates/anyhow) and [serde](https://crates.io/crates/anyhow) are used to deserialize csv input and serialize csv output.
