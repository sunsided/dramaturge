//! Procedural macros for the `dramaturge` dialogue toolkit.
//!
//! This crate provides derive macros that integrate with the dramaturge
//! ecosystem. Currently four macros are available:
//!
//! - `DialogTag` — implements the `DialogTag` trait for an enum,
//!   using `#[tag]` attributes on variants.
//! - `Dramaturge` — a no-op placeholder derive macro.
//! - `FactValue` — generates typed fact helpers and condition builders
//!   for a user-defined fact enum.
//! - `ConditionBuilder` — generates fluent condition construction methods
//!   on a fact enum.
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
//!
//! # `#[derive(FactValue)]`
//!
//! Generates typed fact helpers for a user-defined fact enum. Each variant
//! represents a game-specific fact. Use `#[fact(type)]` to specify the
//! fact value type (defaults to `bool`).
//!
//! ```ignore
//! use dramaturge_macros::FactValue;
//!
//! #[derive(FactValue)]
//! enum GameFacts {
//!     MetHero,                        // bool (default)
//!     QuestStarted,                   // bool
//!     #[fact(int)]
//!     Gold,                           // i64
//!     #[fact(text)]
//!     PlayerName,                     // String
//!     #[fact(float)]
//!     Health,                         // f64
//!     #[fact(entity)]
//!     Companion,                      // EntityId
//! }
//!
//! // Usage:
//! let cond = GameFacts::MetHero.is_true(FactScope::World);
//! let cond = GameFacts::Gold.at_least(FactScope::World, 100);
//! let cond = GameFacts::PlayerName.equals_text(FactScope::World, "Commander");
//! let cond = GameFacts::MetHero.exists(FactScope::Player);
//! ```
//!
//! Generated methods per variant type:
//! - All: `exists(scope)`, `missing(scope)`
//! - Bool: `is_true(scope)`, `is_false(scope)`
//! - Int: `equals_int(scope, i64)`, `at_least(scope, i64)`, `at_most(scope, i64)`
//! - Float: `equals_float(scope, f64)`
//! - Text: `equals_text(scope, String)`
//! - Entity: `equals_entity(scope, EntityId)`
//!
//! # `#[derive(ConditionBuilder)]`
//!
//! Generates fluent condition construction methods on a fact enum,
//! providing a domain-specific API for building condition trees.
//!
//! ```ignore
//! use dramaturge_macros::{FactValue, ConditionBuilder};
//!
//! #[derive(FactValue, ConditionBuilder)]
//! enum GameFacts {
//!     MetHero,
//!     #[fact(int)]
//!     Gold,
//! }
//!
//! // Usage:
//! let cond = GameFacts::all(vec![
//!     GameFacts::MetHero.is_true(FactScope::World),
//!     GameFacts::Gold.at_least(FactScope::World, 100),
//! ]);
//! let cond = GameFacts::any(vec![
//!     GameFacts::MetHero.is_true(FactScope::World),
//! ]);
//! let cond = GameFacts::not(GameFacts::MetHero.is_false(FactScope::World));
//! ```
//!
//! # `#[derive(FactValue)]`
//!
//! Generates typed fact helpers and condition builders for a user-defined
//! fact enum. Each variant becomes a `Fact` constructor and a `Condition`
//! builder method.
//!
//! ```ignore
//! use dramaturge_macros::FactValue;
//!
//! #[derive(FactValue)]
//! enum MyFact {
//!     QuestComplete,
//!     GoldAmount(i64),
//! }
//! ```
//!
//! # `#[derive(ConditionBuilder)]`
//!
//! Generates fluent condition construction methods on a fact enum, enabling
//! `MyFact::QuestComplete.is_true()` style condition building.
//!
//! ```ignore
//! use dramaturge_macros::ConditionBuilder;
//!
//! #[derive(ConditionBuilder)]
//! enum MyFact {
//!     QuestComplete,
//! }
//! ```
//!
//! # `#[derive(Dramaturge)]`
//!
//! Placeholder derive macro. Currently a no-op; reserved for future use.

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_crate::FoundCrate;
use quote::quote;
use syn::{
    Data, DataEnum, DeriveInput, Fields, Meta, MetaNameValue, Token, parse_macro_input,
    punctuated::Punctuated,
};

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

    let crate_path = resolve_dramaturge_model_path();

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
        impl #crate_path::DialogTag for #name {
            fn axis(&self) -> &str {
                match self {
                    #(#axis_arms)*
                }
            }

            fn key(&self) -> &str {
                match self {
                    #(#key_arms)*
                }
            }
        }
    };

    output.into()
}

/// Resolves the path to the `dramaturge-model` crate using `proc_macro_crate`.
/// Falls back to `::dramaturge_model` if resolution fails.
fn resolve_dramaturge_model_path() -> syn::Path {
    match proc_macro_crate::crate_name("dramaturge-model") {
        Ok(FoundCrate::Itself) => syn::parse_quote! { crate },
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            syn::parse_quote! { ::#ident }
        }
        Err(_) => syn::parse_quote! { ::dramaturge_model },
    }
}

/// Parses `#[tag(key = "...")]` from variant attributes using `syn`'s proper parsing.
///
/// Searches the variant's attributes for a `#[tag(...)]` list, then
/// parses it as a `Punctuated<MetaNameValue, Token![,]>` to extract
/// the specified `attr_name` (e.g., `"axis"` or `"key"`). Returns the
/// properly unescaped string value or `None` if not found.
fn extract_tag_attribute(variant: &syn::Variant, attr_name: &str) -> Option<String> {
    for attr in &variant.attrs {
        if !attr.path().is_ident("tag") {
            continue;
        }

        if let Meta::List(meta_list) = &attr.meta {
            let args =
                meta_list.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated);
            if let Ok(args) = args {
                for meta in args {
                    if meta.path.is_ident(attr_name)
                        && let syn::Expr::Lit(expr_lit) = &meta.value
                        && let syn::Lit::Str(lit_str) = &expr_lit.lit
                    {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

/// Derives typed fact helpers for a user-defined fact enum.
///
/// Use `#[fact(type)]` on each variant to specify the fact value type.
/// Supported types: `bool` (default), `int`, `float`, `text`, `entity`.
///
/// Generates:
/// - `fn fact_key(&self) -> &'static str` — returns the snake_case key for each variant
/// - `fn exists(self, scope: FactScope) -> Condition`
/// - `fn missing(self, scope: FactScope) -> Condition`
/// - Type-specific condition methods (see crate docs)
///
/// # Errors
///
/// Returns a compile error if the derive is applied to a non-enum type.
#[proc_macro_derive(FactValue, attributes(fact))]
pub fn derive_fact_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        _ => {
            return syn::Error::new(input.ident.span(), "FactValue can only be derived on enums")
                .to_compile_error()
                .into();
        }
    };

    let crate_path = resolve_dramaturge_model_path();

    let fact_key_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let key = to_snake_case(&variant_name.to_string());
        let variant_pattern = match &variant.fields {
            Fields::Unit => quote! { Self::#variant_name },
            Fields::Unnamed(_) => quote! { Self::#variant_name(..) },
            Fields::Named(_) => quote! { Self::#variant_name { .. } },
        };
        quote! { #variant_pattern => #key, }
    });

    let fact_type_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let fact_type = extract_fact_type(variant);
        let variant_pattern = match &variant.fields {
            Fields::Unit => quote! { Self::#variant_name },
            Fields::Unnamed(_) => quote! { Self::#variant_name(..) },
            Fields::Named(_) => quote! { Self::#variant_name { .. } },
        };
        quote! { #variant_pattern => #fact_type, }
    });

    let exists_impl = quote! {
        pub fn exists(self, scope: #crate_path::FactScope) -> #crate_path::Condition {
            #crate_path::Condition::FactExists {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
            }
        }
    };

    let missing_impl = quote! {
        pub fn missing(self, scope: #crate_path::FactScope) -> #crate_path::Condition {
            #crate_path::Condition::negate(
                #crate_path::Condition::FactExists {
                    scope,
                    fact_id: #crate_path::FactId::new(self.fact_key()),
                }
            )
        }
    };

    let bool_impls = quote! {
        pub fn is_true(self, scope: #crate_path::FactScope) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Bool(true),
            }
        }

        pub fn is_false(self, scope: #crate_path::FactScope) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Bool(false),
            }
        }
    };

    let int_impls = quote! {
        pub fn equals_int(self, scope: #crate_path::FactScope, value: i64) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Int(value),
            }
        }

        pub fn at_least(self, scope: #crate_path::FactScope, value: i64) -> #crate_path::Condition {
            #crate_path::Condition::FactAtLeast {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value,
            }
        }

        pub fn at_most(self, scope: #crate_path::FactScope, value: i64) -> #crate_path::Condition {
            #crate_path::Condition::FactAtMost {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value,
            }
        }
    };

    let float_impls = quote! {
        pub fn equals_float(self, scope: #crate_path::FactScope, value: f64) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Float(value),
            }
        }
    };

    let text_impls = quote! {
        pub fn equals_text(self, scope: #crate_path::FactScope, value: impl Into<std::sync::Arc<str>>) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Text(value.into().to_string()),
            }
        }
    };

    let entity_impls = quote! {
        pub fn equals_entity(self, scope: #crate_path::FactScope, entity: #crate_path::EntityId) -> #crate_path::Condition {
            #crate_path::Condition::FactEquals {
                scope,
                fact_id: #crate_path::FactId::new(self.fact_key()),
                value: #crate_path::FactValue::Entity(entity),
            }
        }
    };

    let output = quote! {
        impl #name {
            pub fn fact_key(&self) -> &'static str {
                match self {
                    #(#fact_key_arms)*
                }
            }

            pub fn fact_value_type(&self) -> &'static str {
                match self {
                    #(#fact_type_arms)*
                }
            }

            #exists_impl
            #missing_impl
            #bool_impls
            #int_impls
            #float_impls
            #text_impls
            #entity_impls
        }
    };

    output.into()
}

/// Derives fluent condition construction methods on a fact enum.
///
/// Generates inherent methods that wrap the `Condition` constructors,
/// providing a domain-specific API for building condition trees.
///
/// # Errors
///
/// Returns a compile error if the derive is applied to a non-enum type.
#[proc_macro_derive(ConditionBuilder)]
pub fn derive_condition_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    if !matches!(&input.data, Data::Enum(_)) {
        return syn::Error::new(
            input.ident.span(),
            "ConditionBuilder can only be derived on enums",
        )
        .to_compile_error()
        .into();
    }

    let crate_path = resolve_dramaturge_model_path();

    let output = quote! {
        impl #name {
            pub fn all(conditions: Vec<#crate_path::Condition>) -> #crate_path::Condition {
                #crate_path::Condition::all(conditions)
            }

            pub fn any(conditions: Vec<#crate_path::Condition>) -> #crate_path::Condition {
                #crate_path::Condition::any(conditions)
            }

            pub fn not(condition: #crate_path::Condition) -> #crate_path::Condition {
                #crate_path::Condition::negate(condition)
            }

            pub fn always() -> #crate_path::Condition {
                #crate_path::Condition::always()
            }

            pub fn never() -> #crate_path::Condition {
                #crate_path::Condition::never()
            }

            pub fn custom(key: impl Into<String>, args: Vec<#crate_path::FactValue>) -> #crate_path::Condition {
                #crate_path::Condition::Custom {
                    key: key.into(),
                    args,
                }
            }

            pub fn has_memory(
                owner: #crate_path::EntityId,
                query: #crate_path::MemoryQuery,
            ) -> #crate_path::Condition {
                #crate_path::Condition::HasMemory { owner, query }
            }

            pub fn has_tag_available(tag: #crate_path::DialogTagKind) -> #crate_path::Condition {
                #crate_path::Condition::HasTagAvailable { tag }
            }

            pub fn alignment_at_least(
                axis: impl Into<std::sync::Arc<str>>,
                pole: impl Into<std::sync::Arc<str>>,
                value: i32,
            ) -> #crate_path::Condition {
                #crate_path::Condition::AlignmentAtLeast {
                    axis: #crate_path::AlignmentAxisId::new(axis),
                    pole: #crate_path::AlignmentPoleId::new(pole),
                    value,
                }
            }

            pub fn relationship_at_least(
                source: #crate_path::EntityId,
                target: #crate_path::EntityId,
                axis: impl Into<std::sync::Arc<str>>,
                value: i32,
            ) -> #crate_path::Condition {
                #crate_path::Condition::RelationshipAtLeast {
                    source,
                    target,
                    axis: #crate_path::RelationshipAxisId::new(axis),
                    value,
                }
            }
        }
    };

    output.into()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn extract_fact_type(variant: &syn::Variant) -> String {
    for attr in &variant.attrs {
        if !attr.path().is_ident("fact") {
            continue;
        }
        if let Meta::List(meta_list) = &attr.meta {
            let tokens = meta_list.tokens.to_string();
            let trimmed = tokens.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Meta::Path(_) = &attr.meta {
            return "bool".to_string();
        }
    }
    "bool".to_string()
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
