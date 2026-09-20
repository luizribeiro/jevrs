use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::{attrs, indexed};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_attrs = attrs::container(&input.attrs)?;
    let source = indexed::variants(input, "Levels", "level", "at least two variants")?;
    let count = source.len();
    if count < 2 {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "`Levels` requires at least 2 variants; `{}` has {count}",
                input.ident
            ),
        ));
    }
    if count > 10 {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "`Levels` supports at most 10 variants; `{}` has {count}",
                input.ident
            ),
        ));
    }
    let mut variants = Vec::with_capacity(source.len());
    let mut descriptions = Vec::with_capacity(source.len());
    for variant in source {
        let parsed = attrs::variant(&variant.attrs, false)?;
        let Some(description) = parsed.description else {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                format!(
                    "level variant `{}` needs a description; add a doc comment or `#[jev(desc = \"…\")]`",
                    variant.ident
                ),
            ));
        };
        variants.push(&variant.ident);
        descriptions.push(description);
    }

    let ident = &input.ident;
    let path = enum_attrs.crate_path;
    let indexed = indexed::implementation(input, &path, &variants);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        const _: () = assert!(#count >= 2 && #count <= 10);

        #indexed

        impl #impl_generics #path::Levels for #ident #ty_generics #where_clause {
            const N: usize = #count;
            type Map<T: ::core::marker::Send + ::core::marker::Sync + 'static> =
                #path::ArrayMap<Self, T, #count>;

            fn description(self) -> &'static str {
                match self {
                    #(Self::#variants => #descriptions),*
                }
            }

            fn from_index(index: usize) -> ::core::option::Option<Self> {
                <Self as #path::Indexed>::all().get(index).copied()
            }

            fn map_from_fn<T: ::core::marker::Send + ::core::marker::Sync + 'static>(
                mut f: impl ::core::ops::FnMut(Self) -> T,
            ) -> Self::Map<T> {
                #path::ArrayMap::new([#(f(Self::#variants)),*])
            }
        }
    })
}
