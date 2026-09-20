use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, punctuated::Punctuated, token::Comma};

use crate::attrs::{self, QuestionKind};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let container = attrs::container(&input.attrs)?;
    let fields = named_fields(input)?;

    let mut parsed_fields = Vec::with_capacity(fields.len());
    let mut ids = HashMap::with_capacity(fields.len());
    for field in fields {
        let parsed = attrs::question_field(field)?;
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "`Questions` requires named fields"))?;
        if let Some(previous) = ids.insert(parsed.id.clone(), ident.to_string()) {
            return Err(syn::Error::new(
                parsed.id_span,
                format!(
                    "duplicate question id `{}` from `{previous}`; give `{ident}` a unique `id`",
                    parsed.id
                ),
            ));
        }
        parsed_fields.push((ident, &field.ty, parsed));
    }

    let name = &input.ident;
    let vis = &input.vis;
    let handles_name = format_ident!("{name}Handles");
    let answers_name = format_ident!("{name}Answers");
    let path = container.crate_path;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let field_names = parsed_fields.iter().map(|(ident, _, _)| *ident);
    let field_types = parsed_fields.iter().map(|(_, ty, _)| *ty);
    let answer_field_names = field_names.clone();
    let answer_field_types = field_types.clone();
    let builds = parsed_fields.iter().map(|(ident, ty, attrs)| {
        let id = &attrs.id;
        let instructions = &attrs.instructions;
        let call = match attrs.kind {
            QuestionKind::Noul => {
                if let (Some(yes), Some(no)) = (&attrs.yes, &attrs.no) {
                    quote!(questions.noul_with(#id, #instructions, #yes, #no)?)
                } else {
                    quote!(questions.noul(#id, #instructions)?)
                }
            }
            QuestionKind::Choice => quote!(questions.choice(#id, #instructions)?),
            QuestionKind::Score => quote!(questions.score(#id, #instructions)?),
        };
        quote!(let #ident: #path::Handle<#ty> = #call;)
    });
    let handle_values = field_names.clone();
    let cloned_answers = parsed_fields.iter().map(
        |(ident, _, _)| quote!(#ident: ::core::clone::Clone::clone(answers.get(handles.#ident))),
    );
    let default_fields = parsed_fields
        .iter()
        .map(|(ident, _, _)| quote!(#ident: ::core::default::Default::default()));
    let read_fields = parsed_fields
        .iter()
        .map(|(ident, _, _)| quote!(&value.#ident));

    Ok(quote! {
        #[doc = concat!("Typed handles generated for [`", stringify!(#name), "`].")]
        #[derive(Clone, Debug)]
        #vis struct #handles_name #generics {
            #(
                #[doc = concat!("Handle for `", stringify!(#field_names), "`.")]
                #vis #field_names: #path::Handle<#field_types>,
            )*
        }

        #[doc = concat!("Typed answers generated for [`", stringify!(#name), "`].")]
        #[derive(Clone, Debug)]
        #vis struct #answers_name #generics {
            #(
                #[doc = concat!("Answer for `", stringify!(#answer_field_names), "`.")]
                #vis #answer_field_names: <#answer_field_types as #path::Question>::Answer,
            )*
        }

        const _: () = {
            #[allow(dead_code)]
            fn _jevrs_keep_alive #impl_generics () -> #name #ty_generics #where_clause {
                let value = #name { #(#default_fields),* };
                let _ = (#(#read_fields),*);
                value
            }
        };

        impl #impl_generics #path::QuestionSet for #name #ty_generics #where_clause {
            type Handles = #handles_name #ty_generics;
            type Answers = #answers_name #ty_generics;

            fn questions() -> ::core::result::Result<(#path::Questions, Self::Handles), #path::Error> {
                let mut questions = #path::Questions::new();
                #(#builds)*
                ::core::result::Result::Ok((questions, #handles_name { #(#handle_values),* }))
            }

            fn answers(handles: &Self::Handles, answers: &#path::Answers) -> Self::Answers {
                #answers_name { #(#cloned_answers),* }
            }
        }
    })
}

fn named_fields(input: &DeriveInput) -> syn::Result<&Punctuated<Field, Comma>> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "`Questions` can only be derived for a struct with named fields",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &data.fields,
            "`Questions` requires a struct with named fields",
        ));
    };
    if fields.named.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "`Questions` requires at least one question field",
        ));
    }
    Ok(&fields.named)
}
