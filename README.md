# bitflate-rs

Simple helper macro for previewing the bitfield layout of your structures!

Trying to pack game/input/network flags without guessing at offsets? Want to validate your structs at compile time instead of runtime? This crate is for you!

## Install

```toml
# Cargo.toml
[dependencies]
bitflate-rs = "0.2.0"
bilge = "0.3.0"
```

```rust
// Using bitflate-rs
use bitflate_rs::prelude::*;
```
`bilge` is required in your crate if you use `bitflate_bits` / `bitflate_enum`.

## Macros

- `#[bitflate]` -> Automagically computes bit layout and padding for repr(C)
- `#[bitflate_bits(N)]` -> packed bitfield structs using the `bilge` crate.
- `#[bitflate_enum(N)]` -> packed enums with a fixed bit width.

Optional feature
- `#[podflate]` -> `repr(C)` structs that must be `bytemuck::Pod` + `Zeroable`
```toml
[dependencies]
bitflate-rs = { version = "0.2.0", features = ["podflate"] }
```

## Examples

### 1) Normal `repr(C)` layout checking

```rust
use bitflate_rs::prelude::*;

#[bitflate]
#[repr(C)]
// Hover me to see my layout!
struct PacketHeader {
    tag: u8,
    sequence: u16,
    flags: u8,
    checksum: u32,
}
```

### 2) Packed bits (common use case)

```rust
use bitflate_rs::prelude::*;

#[bitflate_enum(3)]
enum InputMode {
    None = 0,
    Tap = 1,
    Hold = 2,
    Release = 3,
    Dash = 4,
    Glide = 5,
    Counter = 6,
    Burst = 7,
}

#[bitflate_bits(16)]
#[derive(FromBits)]
struct PackedHeader {
    #[bits(3)]
    mode: InputMode,
    flag: bool,
    value: u4,
    extra: u8,
}
```

## Notes

- If you want **less codegen**, do not derive `TryFromBits` unless you need fallible parsing.
- `FromBits` does not require deriving `TryFromBits`.
- If bilge expansion asks for `TryFrom` in scope, `use bitflate_rs::prelude::*;` already brings it in.
- For nested packed fields, add `#[bits(N)]` so preview/validation can compute widths.

## Examples and Tests

```bash
cargo run --example layout_demo
cargo run --example bitpacked_demo
cargo run --example complex_nested_demo
cargo run --example podflate_demo --features podflate
```
```bash
cargo test
```
