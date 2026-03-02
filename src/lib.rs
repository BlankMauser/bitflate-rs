//! `bitflate-rs` helps you visualize and validate struct/bitfield layout at compile time.
//!
//! Quick start:
//! ```rust
//! use bitflate_rs::prelude::*;
//! ```
//!
//! `#[bitflate]` for `repr(C)` layout previews:
//! ```rust
//! use bitflate_rs::prelude::*;
//!
//! #[bitflate(prefix = "raw_")]
//! #[repr(C)]
//! struct PacketHeader {
//!     tag: u8,
//!     sequence: u16,
//!     flags: u8,
//!     checksum: u32,
//! }
//! ```
//!
//! Disable generated accessors when you want to write your own:
//! ```rust
//! use bitflate_rs::prelude::*;
//!
//! #[bitflate(no_accessors)]
//! #[repr(C)]
//! struct ManualApi {
//!     value: u32,
//! }
//! ```
//!
//! For nested/custom fields in a `#[bitflate]` struct, provide byte-size hints:
//! ```rust
//! use bitflate_rs::prelude::*;
//!
//! #[bitflate]
//! #[repr(C)]
//! struct HdrCat {
//!     valid_frames: [u8; 32],
//! }
//!
//! #[bitflate]
//! #[repr(C)]
//! struct InputModule {
//!     owner: *mut u8,
//!     #[layout(bytes = 32, align = 1)]
//!     hdr_cat: HdrCat,
//!     hold_all: bool,
//! }
//! ```
//!
//! Packed bitfields and enums are bilge-backed:
//! ```rust
//! use bitflate_rs::prelude::*;
//!
//! #[bitflate_enum(2)]
//! enum Mode {
//!     A = 0,
//!     B = 1,
//!     C = 2,
//!     D = 3,
//! }
//!
//! #[bitflate_bits(8)]
//! #[derive(FromBits)]
//! struct Packed {
//!     #[bits(2)]
//!     mode: Mode,
//!     x: bool,
//!     y: bool,
//!     rest: u4,
//! }
//! ```
//!
//! Notes:
//! - `#[bitflate_bits(N)]` and `#[bitflate_enum(N)]` use **bits**, not bytes.
//! - `#[bitflate]` layout hints use **bytes**: `#[layout(bytes = N, align = M)]`.
//! - If you use bitfield macros in another crate, add `bilge` there too.
//!
extern crate self as bitflate_rs;

#[cfg(feature = "bilge")]
pub use bilge;
pub use bitflate_rs_macros::bitflate;
pub use bitflate_rs_macros::bitflate_bits;
pub use bitflate_rs_macros::bitflate_enum;
#[cfg(feature = "podflate")]
pub use bitflate_rs_macros::podflate;
#[cfg(feature = "podflate")]
pub use bytemuck;

#[inline]
pub const fn align_up(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + (align - 1)) & !(align - 1)
    }
}

pub mod prelude {
    pub use crate::{bitflate, bitflate_bits, bitflate_enum};
    #[cfg(feature = "podflate")]
    pub use crate::podflate;
    #[cfg(feature = "bilge")]
    pub use bilge::prelude::*;
    pub use core::convert::TryFrom;
}
