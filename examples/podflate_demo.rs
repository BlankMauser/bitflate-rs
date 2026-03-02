#[cfg(feature = "podflate")]
mod demo {
    use bitflate_rs::podflate;

    #[podflate]
    struct PodPacket {
        tag: u32,
        seq: u16,
        flags: u16,
    }

    pub fn run() {
        let mut packet = PodPacket {
            tag: 0xDEAD_BEEF,
            seq: 7,
            flags: 0,
        };

        packet.set_flags(0b1010);

        println!("flags={}", packet.get_flags());
        println!("size={} bytes", core::mem::size_of::<PodPacket>());
    }
}

#[cfg(feature = "podflate")]
fn main() {
    demo::run();
}

#[cfg(not(feature = "podflate"))]
fn main() {
    println!("Enable the `podflate` feature to run this example.");
}
