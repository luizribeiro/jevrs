use proc_macro2::Span;
use syn::{Attribute, Expr, ExprLit, Field, Lit, LitStr, Meta, Path};

pub(crate) struct ContainerAttrs {
    pub(crate) crate_path: Path,
}

pub(crate) struct VariantAttrs {
    pub(crate) key: Option<(String, Span)>,
    pub(crate) description: Option<String>,
}

pub(crate) enum QuestionKind {
    Noul,
    Choice,
    Score,
}

pub(crate) struct FieldAttrs {
    pub(crate) id: String,
    pub(crate) id_span: Span,
    pub(crate) instructions: String,
    pub(crate) kind: QuestionKind,
    pub(crate) yes: Option<String>,
    pub(crate) no: Option<String>,
}

pub(crate) fn container(attrs: &[Attribute]) -> syn::Result<ContainerAttrs> {
    let mut crate_path = syn::parse_str("::jevrs")?;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("jev")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                let value = meta.value()?.parse::<LitStr>()?;
                crate_path = syn::parse_str(&value.value()).map_err(|_| {
                    syn::Error::new(value.span(), "`crate` must be a valid Rust path string")
                })?;
                Ok(())
            } else {
                Err(meta.error("unknown `jev` attribute; use `crate = \"path\"` on the enum"))
            }
        })?;
    }
    Ok(ContainerAttrs { crate_path })
}

pub(crate) fn variant(attrs: &[Attribute], allow_key: bool) -> syn::Result<VariantAttrs> {
    let mut key = None;
    let mut description = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("jev")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                if !allow_key {
                    return Err(
                        meta.error("`key` is only valid for Options; remove it from this level")
                    );
                }
                let value = meta.value()?.parse::<LitStr>()?;
                if value.value().is_empty() {
                    return Err(syn::Error::new(
                        value.span(),
                        "option key must not be empty; provide a non-empty `key`",
                    ));
                }
                key = Some((value.value(), value.span()));
                Ok(())
            } else if meta.path.is_ident("desc") {
                description = Some(meta.value()?.parse::<LitStr>()?.value());
                Ok(())
            } else {
                Err(meta.error(if allow_key {
                    "unknown `jev` attribute; use `key = \"…\"` or `desc = \"…\"`"
                } else {
                    "unknown `jev` attribute; use `desc = \"…\"`"
                }))
            }
        })?;
    }

    if description.is_none() {
        let lines = attrs.iter().filter_map(doc_line).collect::<Vec<_>>();
        if !lines.is_empty() {
            description = Some(lines.join(" "));
        }
    }

    Ok(VariantAttrs { key, description })
}

pub(crate) fn question_field(field: &Field) -> syn::Result<FieldAttrs> {
    let ident = field.ident.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(field, "`Questions` requires named fields; name this field")
    })?;
    let mut id = ident.to_string();
    let mut id_span = ident.span();
    let mut kind = None;
    let mut yes = None;
    let mut no = None;

    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("jev"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                let value = meta.value()?.parse::<LitStr>()?;
                id = value.value();
                id_span = value.span();
                return Ok(());
            }
            if meta.path.is_ident("yes") {
                yes = Some(meta.value()?.parse::<LitStr>()?.value());
                return Ok(());
            }
            if meta.path.is_ident("no") {
                no = Some(meta.value()?.parse::<LitStr>()?.value());
                return Ok(());
            }
            let parsed_kind = if meta.path.is_ident("noul") {
                QuestionKind::Noul
            } else if meta.path.is_ident("choice") {
                QuestionKind::Choice
            } else if meta.path.is_ident("score") {
                QuestionKind::Score
            } else {
                return Err(meta.error(
                    "unknown `jev` attribute; use `noul`, `choice`, `score`, `id`, `yes`, or `no`",
                ));
            };
            if kind.is_some() {
                return Err(meta.error(
                    "question field must have exactly one of `noul`, `choice`, or `score`",
                ));
            }
            let instructions = meta.value()?.parse::<LitStr>()?.value();
            kind = Some((parsed_kind, instructions));
            Ok(())
        })?;
    }

    let Some((kind, instructions)) = kind else {
        return Err(syn::Error::new_spanned(
            field,
            "question field requires exactly one of `#[jev(noul = \"…\")]`, `#[jev(choice = \"…\")]`, or `#[jev(score = \"…\")]`",
        ));
    };
    if yes.is_some() != no.is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "`yes` and `no` must be provided together",
        ));
    }
    if (yes.is_some() || no.is_some()) && !matches!(kind, QuestionKind::Noul) {
        return Err(syn::Error::new_spanned(
            field,
            "`yes` and `no` are only valid for a `noul` question",
        ));
    }

    Ok(FieldAttrs {
        id,
        id_span,
        instructions,
        kind,
        yes,
        no,
    })
}

fn doc_line(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("doc") {
        return None;
    }
    let Meta::NameValue(name_value) = &attr.meta else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = &name_value.value
    else {
        return None;
    };
    Some(value.value().trim().to_owned())
}
