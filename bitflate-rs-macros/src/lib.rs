use proc_macro::TokenStream;

mod bitflaters;
mod podflaters;

#[proc_macro_attribute]
pub fn bitflate(args: TokenStream, input: TokenStream) -> TokenStream {
    bitflaters::bitflate(args, input)
}

#[proc_macro_attribute]
pub fn bitflate_bits(args: TokenStream, input: TokenStream) -> TokenStream {
    bitflaters::bitflate_bits(args, input)
}

#[proc_macro_attribute]
pub fn bitflate_enum(args: TokenStream, input: TokenStream) -> TokenStream {
    bitflaters::bitflate_enum(args, input)
}

#[proc_macro_attribute]
pub fn podflate(args: TokenStream, input: TokenStream) -> TokenStream {
    podflaters::podflate(args, input)
}
