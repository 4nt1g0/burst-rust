# burst-rust
This project is intended to become a library for burstcoin relevant tools written in rust.

## online-burst
As a proof-of-concept this library includes an online burst miner that requires no hdd but submits nonces it calculates on the fly. 

Create a `Settings.toml` file analogue to the `Settings-default.toml` and run

`cargo run --bin online-burst --release`
