## Dorico-SpeedEditor proof-of-concept demo

Simple proof-of-concept solution for controlling music notation software Steinberg Dorico via its Remote API using Blackmagic's Davini Resolve SpeedEditor.

Written in Rust, using Akira Kamikura's bmd-speededitor crate: 
https://github.com/camikura/bmd-speededitor-rs

Basic functionality:
- activate Dorico Write mode
- activate note entry mode
- select note durations
- activate slurs beginning/end
- navigation using jog wheel

Not intended for anything else than a simple demo!
(Might become useful inte future.)

### Usage

- Clone this repo
- Navigate into repo directory and run `cargo build` to install and build the program.
- Run it using `cargo run` or by running the executable found in the `/target/debug/` directory

### Problems

Only tried it on my windows machine. Sometimes, the software loses its connection with the SpeedEditor unit.
I don't have a clue why.


