use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Fields, ItemStruct, Token};

pub fn podflate(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[podflate] does not accept arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut item = parse_macro_input!(input as ItemStruct);
    if !matches!(item.fields, Fields::Named(_)) {
        return syn::Error::new_spanned(item, "#[podflate] only supports named structs")
            .to_compile_error()
            .into();
    }

    if !has_attr(&item.attrs, "repr") {
        item.attrs.push(syn::parse_quote!(#[repr(C)]));
    }

    add_missing_derives(
        &mut item.attrs,
        &["Copy", "Clone", "Zeroable", "Pod"],
    );

    if !has_attr(&item.attrs, "bitflate") {
        item.attrs.push(syn::parse_quote!(#[::bitflate_rs::bitflate]));
    }

    let name = item.ident.clone();
    quote! {
        #item

        const _: () = {
            fn assert_pod<T: ::bitflate_rs::bytemuck::Pod>() {}
            fn assert_zeroable<T: ::bitflate_rs::bytemuck::Zeroable>() {}
            let _ = assert_pod::<#name> as fn();
            let _ = assert_zeroable::<#name> as fn();
        };
    }
    .into()
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn add_missing_derives(attrs: &mut Vec<syn::Attribute>, wanted: &[&str]) {
    let existing = collect_derive_idents(attrs);
    let mut missing = Vec::new();
    for want in wanted {
        if !existing.contains(*want) {
            missing.push(*want);
        }
    }

    if missing.is_empty() {
        return;
    }

    let mut derive_paths: Vec<syn::Path> = Vec::new();
    for m in missing {
        let text = match m {
            "Zeroable" | "Pod" => format!("::bitflate_rs::bytemuck::{m}"),
            _ => m.to_string(),
        };
        let path: syn::Path = syn::parse_str(&text).expect("valid derive path");
        derive_paths.push(path);
    }

    attrs.push(syn::parse_quote!(#[derive(#(#derive_paths),*)]));
}

fn collect_derive_idents(attrs: &[syn::Attribute]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        if let Ok(paths) = attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
        {
            for path in paths {
                if let Some(seg) = path.segments.last() {
                    out.insert(seg.ident.to_string());
                }
            }
        }
    }
    out
}
