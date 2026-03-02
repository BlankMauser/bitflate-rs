use bitflate_rs::bitflate;

#[bitflate]
#[repr(C)]
struct PacketHeader {
    tag: u8,
    sequence: u16,
    flags: u8,
    checksum: u32,
}

#[bitflate]
#[repr(C)]
struct PartialBits {
    a: u8,
    #[bits(3)]
    tiny: u8,
    b: u16,
}

fn main() {
    let mut header = PacketHeader {
        tag: 0xAB,
        sequence: 7,
        flags: 0,
        checksum: 0,
    };

    header.set_flags(0b1010_0001);
    header.set_checksum(0xDEAD_BEEF);

    println!(
        "tag={:#X} seq={} flags={:#010b} checksum={:#X}",
        header.get_tag(),
        header.get_sequence(),
        header.get_flags(),
        header.get_checksum(),
    );
    println!("size={} bytes", core::mem::size_of::<PacketHeader>());
    let _partial = PartialBits { a: 1, tiny: 5, b: 2 };
}
