use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Path, punctuated::Punctuated, token::Comma};

pub(crate) fn variants<'a>(
    input: &'a DeriveInput,
    derive_name: &str,
    noun: &str,
    minimum: &str,
) -> syn::Result<&'a Punctuated<syn::Variant, Comma>> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            format!("`{derive_name}` can only be derived for an enum; use a unit-variant enum"),
        ));
    };
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("`{derive_name}` requires {minimum}; add {noun} variants"),
        ));
    }
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                format!(
                    "{noun} variant `{}` must be a unit variant; remove its fields",
                    variant.ident
                ),
            ));
        }
    }
    Ok(&data.variants)
}

pub(crate) fn implementation(input: &DeriveInput, path: &Path, variants: &[&Ident]) -> TokenStream {
    let ident = &input.ident;
    let indexes = 0..variants.len();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #path::Indexed for #ident #ty_generics #where_clause {
            fn all() -> &'static [Self] {
                &[#(Self::#variants),*]
            }

            fn index(self) -> usize {
                match self {
                    #(Self::#variants => #indexes),*
                }
            }
        }
    }
}
