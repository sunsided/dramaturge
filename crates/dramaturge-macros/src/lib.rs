//! Procedural macros for the `dramaturge` dialogue toolkit.
//!
//! This crate is a placeholder. Derive macros and attribute macros will be
//! added here in follow-up iterations as the domain model stabilises.

extern crate proc_macro;

use proc_macro::TokenStream;

/// A placeholder derive macro provided by `dramaturge-macros`.
///
/// Currently a no-op. Future versions will generate trait implementations
/// relevant to the `dramaturge` domain model.
#[proc_macro_derive(Dramaturge)]
pub fn derive_dramaturge(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
