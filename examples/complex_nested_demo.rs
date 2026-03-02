use bilge::prelude::*;
use bitflate_rs::{bitflate, bitflate_bits, bitflate_enum};

#[bitflate_enum(4)]
enum TriggerKind {
    None = 0,
    Tap = 1,
    Hold = 2,
    Combo = 3,
    Dash = 4,
    Glide = 5,
    Counter = 6,
    Burst = 7,
    MacroA = 8,
    MacroB = 9,
    MacroC = 10,
    MacroD = 11,
    MacroE = 12,
    MacroF = 13,
    MacroG = 14,
    MacroH = 15,
}

#[bitflate_bits(8)]
#[derive(FromBits)]
struct InputSlot {
    #[bits(4)]
    kind: TriggerKind,
    sticky: bool,
    turbo: bool,
    layer: u2,
}

#[bitflate_bits(32)]
#[derive(FromBits)]
struct InputProfile {
    #[bits(8)]
    a: InputSlot,
    #[bits(8)]
    b: InputSlot,
    deadzone: u7,
    invert_y: bool,
    sensitivity: u8,
}

#[bitflate]
#[repr(C)]
struct ProfileBlob {
    magic: u16,
    version: u8,
    #[bits(32)]
    packed: InputProfile,
    #[bits(4)]
    nibble: u8,
    checksum: u16,
}

fn main() {
    let profile = InputProfile::new(
        InputSlot::new(TriggerKind::Tap, true, false, u2::new(2)),
        InputSlot::new(TriggerKind::Burst, false, true, u2::new(1)),
        u7::new(95),
        true,
        u8::new(140),
    );

    let _blob = ProfileBlob {
        magic: 0xB17F,
        version: 3,
        packed: profile,
        nibble: 7,
        checksum: 0xAA55,
    };

    println!(
        "input profile size={} bytes",
        core::mem::size_of::<InputProfile>()
    );
    println!(
        "profile blob size={} bytes",
        core::mem::size_of::<ProfileBlob>()
    );
}
