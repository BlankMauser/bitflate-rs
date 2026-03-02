use bilge::prelude::*;
use bitflate_rs::{bitflate, bitflate_bits, bitflate_enum};
#[cfg(feature = "podflate")]
use bitflate_rs::podflate;

#[bitflate]
#[repr(C)]
struct Demo {
    a: u8,
    b: u32,
    c: u8,
}

#[test]
fn generated_accessors_work() {
    let mut demo = Demo { a: 1, b: 2, c: 3 };
    assert_eq!(*demo.get_a(), 1);
    *demo.get_b_mut() = 4;
    demo.set_c(5);
    assert_eq!(*demo.get_b(), 4);
    assert_eq!(*demo.get_c(), 5);
}

#[test]
fn layout_mentions_padding() {
    let _ = Demo { a: 1, b: 2, c: 3 };
}

#[bitflate_bits(8)]
#[derive(FromBits)]
struct SmallBits {
    lo: u3,
    hi: u5,
}

#[bitflate_bits(16)]
#[derive(FromBits)]
struct NestedBits {
    kind: u4,
    #[bits(8)]
    payload: SmallBits,
    flags: u4,
}

#[test]
fn bitflate_bits_supports_nested_and_arbitrary_widths() {
    let mut v = NestedBits::new(
        u4::new(0b1010),
        SmallBits::new(u3::new(0b101), u5::new(0b11111)),
        u4::new(0b0011),
    );
    assert_eq!(u8::from(v.kind()), 0b1010);
    assert_eq!(u8::from(v.payload().lo()), 0b101);
    v.set_flags(u4::new(0b1100));
    assert_eq!(u8::from(v.flags()), 0b1100);

    let _ = NestedBits::new(
        u4::new(0b0001),
        SmallBits::new(u3::new(0b001), u5::new(0b00011)),
        u4::new(0b0010),
    );
}

#[bitflate_enum(3)]
enum ActionKind {
    None = 0,
    Tap = 1,
    Hold = 2,
    Combo = 3,
    Dash = 4,
    Glide = 5,
    Counter = 6,
    Burst = 7,
}

#[bitflate_bits(24)]
#[derive(FromBits)]
struct ComplexPacket {
    #[bits(3)]
    kind: ActionKind,
    level: u5,
    #[bits(16)]
    nested: NestedBits,
}

#[test]
fn nested_enum_bitfield_layout_and_size_are_checked() {
    let packet = ComplexPacket::new(
        ActionKind::Burst,
        u5::new(17),
        NestedBits::new(
            u4::new(0b1010),
            SmallBits::new(u3::new(0b001), u5::new(0b11100)),
            u4::new(0b0110),
        ),
    );
    assert_eq!(u8::from(packet.level()), 17);
    assert_eq!(u8::from(u3::from(packet.kind())), 7);
    let _ = packet;
}

const _: () = {
    assert!(core::mem::size_of::<Demo>() == 12);
    assert!(<NestedBits as bilge::Bitsized>::BITS == 16);
    assert!(<ComplexPacket as bilge::Bitsized>::BITS == 24);
    assert!(<ActionKind as bilge::Bitsized>::BITS == 3);
};

#[cfg(feature = "podflate")]
#[podflate]
struct PodHeader {
    magic: u32,
    count: u16,
    flags: u16,
}

#[cfg(feature = "podflate")]
#[test]
fn podflate_generates_pod_and_layout() {
    let mut p = PodHeader {
        magic: 0xAA55AA55,
        count: 4,
        flags: 0,
    };
    p.set_flags(3);
    assert_eq!(*p.get_flags(), 3);
    let bytes = bytemuck::bytes_of(&p);
    assert_eq!(bytes.len(), core::mem::size_of::<PodHeader>());
}
