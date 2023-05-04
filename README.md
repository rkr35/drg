# drg

## Primary goal
Produce a .DLL that players can use to write native Unreal Engine modifications for [Deep Rock Galactic](https://www.deeprockgalactic.com/).

## Secondary goals
Use these restrictions to learn new things:
* No Rust standard library (enforced through `#![no_std]`)
* No third-party crate dependencies
* No heap allocations
* No panic branches (enforced through unlinkable panic_handler)
