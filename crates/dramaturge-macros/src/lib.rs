//! Procedural macros for the `dramaturge` dialogue toolkit.
//!
//! This crate provides derive macros that integrate with the dramaturge
//! ecosystem. Currently two macros are available:
//!
//! - `DialogTag` — implements the `DialogTag` trait for an enum,
//!   using `#[tag]` attributes on variants.
//! - `Dramaturge` — a no-op placeholder derive macro.
//!
//! # `#[derive(DialogTag)]`
//!
//! Use the `#[tag(axis = "...", key = "...")]` attribute on enum variants
//! to specify the axis and key for each tag. If a variant lacks the
//! attribute, the axis or key defaults to an empty string.
//!
//! ```ignore
//! use dramaturge_macros::DialogTag;
//!
//! #[derive(DialogTag)]
//! enum MyTag {
//!     #[tag(axis = "moral", key = "good")]
//!     Good,
//!     #[tag(axis = "moral", key = "evil")]
//!     Evil,
//! }
//! ```

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Fields, Meta, parse_macro_input};

/// Derives the `DialogTag` trait for an enum.
///
/// Each variant should use the `#[tag(axis = "...", key = "...")]` attribute
/// to specify its axis and key. Variants without the attribute default to
/// an empty string for the missing attribute.
///
/// # Errors
///
/// Returns a compile error if the derive is applied to a non-enum type.
#[proc_macro_derive(DialogTag, attributes(tag))]
pub fn derive_dialog_tag(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        _ => {
            return syn::Error::new(input.ident.span(), "DialogTag can only be derived on enums")
                .to_compile_error()
                .into();
        }
    };

    let axis_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let axis = extract_tag_attribute(variant, "axis").unwrap_or_default();
        let variant_pattern = match &variant.fields {
            Fields::Unit => quote! { Self::#variant_name },
            Fields::Unnamed(_) => {
                quote! { Self::#variant_name(..) }
            }
            Fields::Named(_) => {
                quote! { Self::#variant_name { .. } }
            }
        };
        quote! {
            #variant_pattern => #axis,
        }
    });

    let key_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let key = extract_tag_attribute(variant, "key").unwrap_or_default();
        let variant_pattern = match &variant.fields {
            Fields::Unit => quote! { Self::#variant_name },
            Fields::Unnamed(_) => {
                quote! { Self::#variant_name(..) }
            }
            Fields::Named(_) => {
                quote! { Self::#variant_name { .. } }
            }
        };
        quote! {
            #variant_pattern => #key,
        }
    });

    let output = quote! {
        impl ::dramaturge_model::DialogTag for #name {
            fn axis(&self) -> &'static str {
                match self {
                    #(#axis_arms)*
                }
            }

            fn key(&self) -> &'static str {
                match self {
                    #(#key_arms)*
                }
            }
        }
    };

    output.into()
}

/// Parses `#[tag(key = "...")]` from variant attributes.
///
/// Searches the variant's attributes for a `#[tag(...)]` list, then
/// looks for the specified `attr_name` (e.g., `"axis"` or `"key"`)
/// followed by `= "value"`. Returns the string value or `None` if
/// not found.
fn extract_tag_attribute(variant: &syn::Variant, attr_name: &str) -> Option<String> {
    for attr in &variant.attrs {
        if !attr.path().is_ident("tag") {
            continue;
        }

        if let Meta::List(meta_list) = &attr.meta {
            let tokens = meta_list.tokens.clone();
            let mut iter = tokens.into_iter();
            while let Some(token) = iter.next() {
                if let proc_macro2::TokenTree::Ident(ident) = &token
                    && *ident == attr_name
                    && let Some(proc_macro2::TokenTree::Punct(_)) = iter.next()
                    && let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next()
                {
                    let lit_str = lit.to_string();
                    return Some(lit_str.trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// No-op placeholder derive macro.
///
/// This derive macro always returns an empty token stream. It exists
/// as a placeholder for future derive functionality that may be added
/// to the dramaturge toolkit.
#[proc_macro_derive(Dramaturge)]
pub fn derive_dramaturge(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn dramaturge_noop() {
        let _: () = ();
    }
}
