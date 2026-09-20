use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::{attrs, indexed};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_attrs = attrs::container(&input.attrs)?;
    let source = indexed::variants(input, "Options", "option", "at least one variant")?;
    let count = source.len();
    if count > 255 {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "`Options` supports at most 255 variants; `{}` has {count}",
                input.ident
            ),
        ));
    }

    let mut variants = Vec::with_capacity(source.len());
    let mut keys = HashMap::with_capacity(source.len());
    let mut key_values = Vec::with_capacity(source.len());
    let mut descriptions = Vec::with_capacity(source.len());
    for variant in source {
        let parsed = attrs::variant(&variant.attrs, true)?;
        let (key, key_span) = parsed
            .key
            .unwrap_or_else(|| (snake_case(&variant.ident.to_string()), variant.ident.span()));
        if let Some(previous) = keys.insert(key.clone(), variant.ident.to_string()) {
            return Err(syn::Error::new(
                key_span,
                format!(
                    "duplicate option key `{key}` from `{previous}`; give this variant a unique `#[jev(key = \"…\")]`"
                ),
            ));
        }
        variants.push(&variant.ident);
        key_values.push(key);
        descriptions.push(parsed.description);
    }

    let ident = &input.ident;
    let path = enum_attrs.crate_path;
    let indexed = indexed::implementation(input, &path, &variants);
    let description_arms = variants
        .iter()
        .zip(descriptions)
        .map(|(variant, description)| {
            if let Some(description) = description {
                quote!(Self::#variant => ::core::option::Option::Some(#description))
            } else {
                quote!(Self::#variant => ::core::option::Option::None)
            }
        });
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        const _: () = assert!(#count >= 1 && #count <= 255);

        #indexed

        impl #impl_generics #path::Options for #ident #ty_generics #where_clause {
            const N: usize = #count;
            type Map<T: ::core::marker::Send + ::core::marker::Sync + 'static> =
                #path::ArrayMap<Self, T, #count>;

            fn key(self) -> &'static str {
                match self {
                    #(Self::#variants => #key_values),*
                }
            }

            fn description(self) -> ::core::option::Option<&'static str> {
                match self {
                    #(#description_arms),*
                }
            }

            fn from_key(key: &str) -> ::core::option::Option<Self> {
                <Self as #path::Indexed>::all()
                    .iter()
                    .copied()
                    .find(|option| <Self as #path::Options>::key(*option) == key)
            }

            fn map_from_fn<T: ::core::marker::Send + ::core::marker::Sync + 'static>(
                mut f: impl ::core::ops::FnMut(Self) -> T,
            ) -> Self::Map<T> {
                #path::ArrayMap::new([#(f(Self::#variants)),*])
            }
        }
    })
}

fn snake_case(name: &str) -> String {
    let chars = name.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(name.len());
    for (index, character) in chars.iter().copied().enumerate() {
        if character.is_uppercase()
            && index > 0
            && (chars[index - 1].is_lowercase()
                || chars[index - 1].is_numeric()
                || chars.get(index + 1).is_some_and(|next| next.is_lowercase()))
        {
            output.push('_');
        }
        output.extend(character.to_lowercase());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::snake_case;

    #[test]
    fn converts_variant_names_to_snake_case() {
        assert_eq!(snake_case("VeryAngry"), "very_angry");
        assert_eq!(snake_case("HTTPServer2XX"), "http_server2_xx");
        assert_eq!(snake_case("A1B"), "a1_b");
    }
}
