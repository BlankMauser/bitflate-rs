use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::cmp::max;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, Attribute, Expr, ExprLit, Fields, ItemEnum, ItemStruct, LitInt, LitStr,
    Meta, Token, Type,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewMode {
    Compact,
    Full,
}

pub fn bitflate(args: TokenStream, input: TokenStream) -> TokenStream {
    let mode = match parse_bitflate_mode(args) {
        Ok(mode) => mode,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(input as ItemStruct);
    expand_bitflate(item, mode).into()
}

fn parse_bitflate_mode(args: TokenStream) -> Result<PreviewMode, syn::Error> {
    if args.is_empty() {
        return Ok(PreviewMode::Compact);
    }
    let mode = syn::parse::<syn::Ident>(args)?;
    if mode == "full" {
        Ok(PreviewMode::Full)
    } else {
        Err(syn::Error::new_spanned(
            mode,
            "expected #[bitflate] or #[bitflate(full)]",
        ))
    }
}

pub fn bitflate_bits(args: TokenStream, input: TokenStream) -> TokenStream {
    let bits_lit = parse_macro_input!(args as LitInt);
    let total_bits = match bits_lit.base10_parse::<usize>() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(bits_lit, "expected usize bit width")
                .to_compile_error()
                .into()
        }
    };

    let mut item = parse_macro_input!(input as ItemStruct);
    let name = item.ident.clone();
    let fields = match &mut item.fields {
        Fields::Named(fields) => &mut fields.named,
        _ => {
            return syn::Error::new_spanned(item, "#[bitflate_bits] only supports named structs")
                .to_compile_error()
                .into()
        }
    };

    let mut preview_lines = Vec::new();
    preview_lines.push(format!("Bit layout for {} (packed)", name));
    preview_lines.push(format!("declared bits: {}", total_bits));
    preview_lines.push(String::new());
    preview_lines.push("Fields:".to_string());

    let mut cursor = 0usize;
    for field in fields.iter_mut() {
        let ident = field.ident.clone().expect("named field");
        let ty = field.ty.clone();
        let bits_override = parse_bits_override(field);
        let bits = match bits_override.or_else(|| bit_width_of_type(&ty)) {
            Some(v) => v,
            None => {
                return syn::Error::new_spanned(
                    &field.ty,
                    "unknown bit width for this field type; add #[bits(N)] on the field",
                )
                .to_compile_error()
                .into()
            }
        };
        let start = cursor;
        let end = cursor + bits - 1;
        preview_lines.push(format!(
            "- {:>12} : {:<20} bits [{:>4}..={:<4}] width {}",
            ident,
            quote!(#ty),
            start,
            end,
            bits
        ));
        cursor += bits;
    }

    let preview_text = preview_lines.join("\n");
    let preview_doc = LitStr::new(
        &format!("bitflate bits preview\n\n```text\n{}\n```", preview_text),
        proc_macro2::Span::call_site(),
    );
    quote! {
        #[doc = #preview_doc]
        #[::bitflate_rs::bilge::bitsize(#bits_lit)]
        #item

        const _: () = {
            assert!(<#name as ::bitflate_rs::bilge::Bitsized>::BITS == #total_bits);
        };
    }
    .into()
}

pub fn bitflate_enum(args: TokenStream, input: TokenStream) -> TokenStream {
    let bits_lit = parse_macro_input!(args as LitInt);
    let total_bits = match bits_lit.base10_parse::<usize>() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(bits_lit, "expected usize bit width")
                .to_compile_error()
                .into()
        }
    };

    let item = parse_macro_input!(input as ItemEnum);
    let name = item.ident.clone();
    let has_from_bits =
        has_derive(&item.attrs, "FromBits") || has_derive(&item.attrs, "TryFromBits");
    let maybe_derive = if has_from_bits {
        quote! {}
    } else {
        quote! { #[derive(::bitflate_rs::bilge::FromBits)] }
    };

    let mut preview_lines = Vec::new();
    preview_lines.push(format!("Enum layout for {} ({} bits)", name, total_bits));
    preview_lines.push(String::new());
    preview_lines.push("Variants:".to_string());

    let mut next_discriminant = 0usize;
    for variant in &item.variants {
        let variant_name = variant.ident.to_string();
        let display_value = if let Some((_, expr)) = &variant.discriminant {
            if let Some(v) = parse_expr_usize(expr) {
                next_discriminant = v.saturating_add(1);
                format!("{v}")
            } else {
                "?".to_string()
            }
        } else {
            let current = next_discriminant;
            next_discriminant = next_discriminant.saturating_add(1);
            format!("{current}")
        };
        preview_lines.push(format!("- {:<20} = {}", variant_name, display_value));
    }

    let preview_text = preview_lines.join("\n");
    let preview_doc = LitStr::new(
        &format!("bitflate enum preview\n\n```text\n{}\n```", preview_text),
        proc_macro2::Span::call_site(),
    );

    quote! {
        #[doc = #preview_doc]
        #[::bitflate_rs::bilge::bitsize(#bits_lit)]
        #maybe_derive
        #item

        const _: () = {
            assert!(<#name as ::bitflate_rs::bilge::Bitsized>::BITS == #total_bits);
        };
    }
    .into()
}

fn expand_bitflate(mut item: ItemStruct, mode: PreviewMode) -> proc_macro2::TokenStream {
    let name = &item.ident;

    if !has_repr_c(&item.attrs) {
        return quote! {
            compile_error!("#[bitflate] requires #[repr(C)] on the struct");
            #item
        };
    }

    let fields = match &mut item.fields {
        Fields::Named(fields) => &mut fields.named,
        _ => {
            return quote! {
                compile_error!("#[bitflate] only supports structs with named fields");
                #item
            };
        }
    };

    let mut getter_setters = Vec::new();
    let mut cursor_updates = Vec::new();
    let mut field_offset_asserts = Vec::new();
    let mut segment_rows = Vec::new();
    let mut byte_owners: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    let mut struct_align = 1usize;
    let mut used_bytes = 0usize;
    let mut preview_supported = true;
    let mut exact_offset_asserts = true;

    for field in fields.iter_mut() {
        let ident = field.ident.clone().expect("named field");
        let ty = field.ty.clone();
        let bits_hint = parse_bits_override(field);
        let get_ident = format_ident!("get_{}", ident);
        let get_mut_ident = format_ident!("get_{}_mut", ident);
        let set_ident = format_ident!("set_{}", ident);
        let field_layout = layout_of_type(&ty).or_else(|| {
            bits_hint.map(|bits| {
                let size = bits_to_bytes(bits);
                (size, size)
            })
        });

        if let Some((field_size, field_align)) = field_layout {
            let aligned_offset = align_up_host(cursor, field_align);

            if aligned_offset > cursor {
                let pad_start = cursor;
                let pad_len = aligned_offset - cursor;
                let pad_end = aligned_offset - 1;
                segment_rows.push(format!(
                    "- [{:>3}..={:<3}] {:<14} {:>3} bytes ({:>4}..={:<4} bits)",
                    pad_start,
                    pad_end,
                    "<padding/free>",
                    pad_len,
                    pad_start * 8,
                    (pad_end * 8) + 7
                ));
                for _ in 0..(aligned_offset - cursor) {
                    byte_owners.push("<padding>".to_string());
                }
            }

            for _ in 0..field_size {
                byte_owners.push(ident.to_string());
            }

            let bit_start = aligned_offset * 8;
            let bit_end = bit_start + (field_size * 8).saturating_sub(1);
            segment_rows.push(format!(
                "- [{:>3}..={:<3}] {:<14} {:>3} bytes ({:>4}..={:<4} bits)  type {:<12} align {}",
                aligned_offset,
                aligned_offset + field_size.saturating_sub(1),
                ident,
                field_size,
                bit_start,
                bit_end,
                quote!(#ty),
                field_align
            ));

            if exact_offset_asserts {
                field_offset_asserts.push(quote! {
                assert!(core::mem::offset_of!(#name, #ident) == #aligned_offset);
                });
            } else {
                field_offset_asserts.push(quote! {
                    let _ = core::mem::offset_of!(#name, #ident);
                });
            }

            cursor = aligned_offset + field_size;
            used_bytes += field_size;
            struct_align = max(struct_align, field_align);
        } else {
            preview_supported = false;
            exact_offset_asserts = false;
            segment_rows.push(format!(
                "- {:>12} : {:<12} <layout preview unavailable for this field type>",
                ident,
                quote!(#ty),
            ));
            field_offset_asserts.push(quote! {
                let _ = core::mem::offset_of!(#name, #ident);
            });
        }

        getter_setters.push(quote! {
            #[inline]
            pub fn #get_ident(&self) -> &#ty {
                    &self.#ident
            }

            #[inline]
            pub fn #get_mut_ident(&mut self) -> &mut #ty {
                    &mut self.#ident
            }

            #[inline]
            pub fn #set_ident(&mut self, value: #ty) {
                    self.#ident = value;
            }
        });

        cursor_updates.push(quote! {
            cursor = ::bitflate_rs::align_up(cursor, core::mem::align_of::<#ty>());
            cursor += core::mem::size_of::<#ty>();
        });
    }

    if preview_supported {
        let computed_size = align_up_host(cursor, struct_align);
        if computed_size > cursor {
            let pad_start = cursor;
            let pad_len = computed_size - cursor;
            let pad_end = computed_size - 1;
            segment_rows.push(format!(
                "- [{:>3}..={:<3}] {:<14} {:>3} bytes ({:>4}..={:<4} bits)",
                pad_start,
                pad_end,
                "<padding/free>",
                pad_len,
                pad_start * 8,
                (pad_end * 8) + 7
            ));
            for _ in 0..(computed_size - cursor) {
                byte_owners.push("<padding>".to_string());
            }
        }
        let free_bytes = computed_size.saturating_sub(used_bytes);

        let mut ascii_layout = String::new();
        ascii_layout.push_str(&format!("Layout for {} (repr(C))\n", name));
        ascii_layout.push_str(&format!(
            "size: {} bytes ({} bits), align: {} bytes\n",
            computed_size,
            computed_size * 8,
            struct_align
        ));
        ascii_layout.push_str(&format!(
            "used: {} bytes, padding/free: {} bytes\n\n",
            used_bytes, free_bytes
        ));
        ascii_layout.push_str("Layout map (in memory order):\n");
        for row in &segment_rows {
            ascii_layout.push_str(row);
            ascii_layout.push('\n');
        }
        if mode == PreviewMode::Full {
            ascii_layout.push_str("\nByte map:\n");
            for (idx, owner) in byte_owners.iter().enumerate() {
                ascii_layout.push_str(&format!("{:>3}: {}\n", idx, owner));
            }
        }
        segment_rows = ascii_layout
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
    } else {
        let mut lines = vec![
            format!("Layout for {} (repr(C))", name),
            "preview: unsupported field type for static visualization".to_string(),
            "hint: add #[bits(N)] on nested/packed fields to provide width".to_string(),
            String::new(),
            "Layout map:".to_string(),
        ];
        lines.extend(segment_rows);
        segment_rows = lines;
    }
    let preview_text = segment_rows.join("\n");
    let preview_doc = LitStr::new(
        &format!("bitflate preview\n\n```text\n{}\n```", preview_text),
        proc_macro2::Span::call_site(),
    );
    quote! {
        #[doc = #preview_doc]
        #item

        impl #name {
            #(#getter_setters)*
        }

        const _: () = {
            #(#field_offset_asserts)*
            let mut cursor = 0usize;
            #(#cursor_updates)*
            let computed = ::bitflate_rs::align_up(cursor, core::mem::align_of::<#name>());
            let actual = core::mem::size_of::<#name>();
            assert!(computed == actual);
        };
    }
}

fn align_up_host(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + (align - 1)) & !(align - 1)
    }
}

fn layout_of_type(ty: &Type) -> Option<(usize, usize)> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let seg = path.path.segments.last()?;
            if !matches!(seg.arguments, syn::PathArguments::None) {
                return None;
            }
            let ident = seg.ident.to_string();
            let primitive_layout = match ident.as_str() {
                "u8" | "i8" | "bool" => (1, 1),
                "u16" | "i16" => (2, 2),
                "u32" | "i32" | "f32" | "char" => (4, 4),
                "u64" | "i64" | "f64" => (8, 8),
                "u128" | "i128" => (16, 16),
                _ => (0, 0),
            };

            if primitive_layout.0 != 0 {
                return Some(primitive_layout);
            }

            if let Some(bits) = parse_arbitrary_int_bits(&ident) {
                let size = bits_to_bytes(bits);
                return Some((size, size));
            }

            None
        }
        Type::Array(array) => layout_of_array(array),
        _ => None,
    }
}

fn layout_of_array(array: &syn::TypeArray) -> Option<(usize, usize)> {
    let (elem_size, elem_align) = layout_of_type(&array.elem)?;
    let Expr::Lit(expr_lit) = &array.len else {
        return None;
    };
    let syn::Lit::Int(len_lit) = &expr_lit.lit else {
        return None;
    };
    let len: usize = parse_lit_usize(len_lit).ok()?;
    let size = elem_size.checked_mul(len)?;
    Some((size, elem_align))
}

fn parse_lit_usize(lit: &LitInt) -> Result<usize, syn::Error> {
    lit.base10_parse::<usize>()
        .map_err(|_| syn::Error::new_spanned(lit, "invalid usize literal"))
}

fn parse_expr_usize(expr: &Expr) -> Option<usize> {
    let Expr::Lit(ExprLit {
        lit: syn::Lit::Int(lit),
        ..
    }) = expr
    else {
        return None;
    };
    lit.base10_parse::<usize>().ok()
}

fn has_derive(attrs: &[Attribute], target: &str) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .any(|attr| {
            attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .map(|paths| {
                    paths.iter().any(|path| {
                        path.segments
                            .last()
                            .map(|seg| seg.ident == target)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
}

fn parse_bits_override(field: &mut syn::Field) -> Option<usize> {
    let mut parsed = None;
    field.attrs.retain(|attr| {
        if !attr.path().is_ident("bits") {
            return true;
        }
        let parsed_here = attr
            .parse_args::<LitInt>()
            .ok()
            .and_then(|lit| lit.base10_parse::<usize>().ok());
        if parsed_here.is_some() {
            parsed = parsed_here;
        }
        false
    });
    parsed
}

fn parse_arbitrary_int_bits(ident: &str) -> Option<usize> {
    let bytes = ident.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let sign = bytes[0];
    if sign != b'u' && sign != b'i' {
        return None;
    }
    let digits = &ident[1..];
    let bits = digits.parse::<usize>().ok()?;
    if (1..=128).contains(&bits) {
        Some(bits)
    } else {
        None
    }
}

fn bits_to_bytes(bits: usize) -> usize {
    if bits <= 8 {
        1
    } else if bits <= 16 {
        2
    } else if bits <= 32 {
        4
    } else if bits <= 64 {
        8
    } else {
        16
    }
}

fn bit_width_of_type(ty: &Type) -> Option<usize> {
    match ty {
        Type::Path(path) if path.qself.is_none() => {
            let seg = path.path.segments.last()?;
            if !matches!(seg.arguments, syn::PathArguments::None) {
                return None;
            }
            let ident = seg.ident.to_string();
            let primitive_bits = match ident.as_str() {
                "bool" => Some(1),
                "u8" | "i8" => Some(8),
                "u16" | "i16" => Some(16),
                "u32" | "i32" | "f32" | "char" => Some(32),
                "u64" | "i64" | "f64" => Some(64),
                "u128" | "i128" => Some(128),
                _ => None,
            };
            primitive_bits.or_else(|| parse_arbitrary_int_bits(&ident))
        }
        Type::Array(array) => {
            let elem_bits = bit_width_of_type(&array.elem)?;
            let Expr::Lit(ExprLit {
                lit: syn::Lit::Int(len_lit),
                ..
            }) = &array.len
            else {
                return None;
            };
            let len = len_lit.base10_parse::<usize>().ok()?;
            elem_bits.checked_mul(len)
        }
        _ => None,
    }
}

fn has_repr_c(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("repr"))
        .any(|attr| {
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map(|items| {
                    items
                        .iter()
                        .any(|meta| matches!(meta, Meta::Path(path) if path.is_ident("C")))
                })
                .unwrap_or(false)
        })
}
