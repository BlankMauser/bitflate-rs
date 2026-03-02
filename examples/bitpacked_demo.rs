use bilge::prelude::*;
use bitflate_rs::{bitflate_bits, bitflate_enum};

#[bitflate_enum(3)]
enum InputMode {
    None = 0,
    Tap = 1,
    Hold = 2,
    Combo = 3,
    Dash = 4,
    Glide = 5,
    Counter = 6,
    Burst = 7,
}

#[bitflate_bits(8)]
#[derive(FromBits)]
struct Footer {
    code: u3,
    done: bool,
    level: u4,
}

#[bitflate_bits(16)]
#[derive(FromBits)]
struct PackedHeader {
    #[bits(3)]
    mode: InputMode,
    flag: bool,
    #[bits(8)]
    footer: Footer,
    tail: u4,
}

fn main() {
    let mut packet = PackedHeader::new(
        InputMode::Dash,
        true,
        Footer::new(u3::new(0b011), true, u4::new(0b1111)),
        u4::new(0b0011),
    );

    packet.set_tail(u4::new(0b1100));

    println!(
        "mode={} tail={}",
        u8::from(u3::from(packet.mode())),
        u8::from(packet.tail())
    );
    println!("size={} bytes", core::mem::size_of::<PackedHeader>());
}
