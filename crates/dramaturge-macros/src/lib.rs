extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Fields, Meta, parse_macro_input};

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
