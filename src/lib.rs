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
}
