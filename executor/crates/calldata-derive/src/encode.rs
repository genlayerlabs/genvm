use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{ContainerAttrs, FieldAttrs};

pub fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let container = ContainerAttrs::from_ast(&input.attrs)?;

    // Add `Encode<__W, Error = __W::Error>` bound to each type parameter.
    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(tp) = param {
            tp.bounds.push(syn::parse_quote!(
                genlayer_calldata::codec::Encode<__W, Error = <__W as genlayer_calldata::Writer>::Error>
            ));
        }
    }

    // Introduce the `__W: Writer` parameter.
    let mut impl_generics_params = generics.params.clone();
    impl_generics_params.insert(0, syn::parse_quote!(__W: genlayer_calldata::Writer));

    let (_, ty_generics, where_clause) = generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => encode_struct(&data.fields)?,
        Data::Enum(data) => encode_enum(data, &container)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Encode cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        impl<#impl_generics_params> genlayer_calldata::codec::Encode<__W> for #name #ty_generics #where_clause {
            type Error = <__W as genlayer_calldata::Writer>::Error;

            fn encode(
                &self,
                __enc: &mut genlayer_calldata::Encoder<__W>,
            ) -> ::core::result::Result<(), <__W as genlayer_calldata::Writer>::Error> {
                #body
            }
        }
    })
}

// ── Structs ──────────────────────────────────────────────────────────

fn encode_struct(fields: &Fields) -> syn::Result<TokenStream> {
    match fields {
        Fields::Named(fields) => encode_named_fields(&fields.named, FieldAccess::SelfDot),
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                let attrs = FieldAttrs::from_ast(&fields.unnamed[0].attrs)?;
                if let Some(func) = &attrs.serialize_with {
                    Ok(quote! { #func(&self.0, __enc) })
                } else {
                    Ok(quote! {
                        genlayer_calldata::codec::Encode::encode(&self.0, __enc)
                    })
                }
            } else {
                let len = fields.unnamed.len() as u64;
                let encodes = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let idx = syn::Index::from(i);
                        let attrs = FieldAttrs::from_ast(&f.attrs)?;
                        if let Some(func) = &attrs.serialize_with {
                            Ok(quote! { #func(&self.#idx, __enc)?; })
                        } else {
                            Ok(quote! {
                                genlayer_calldata::codec::Encode::encode(&self.#idx, __enc)?;
                            })
                        }
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                Ok(quote! {
                    __enc.start_array(#len)?;
                    #(#encodes)*
                    Ok(())
                })
            }
        }
        Fields::Unit => Ok(quote! { __enc.push_null() }),
    }
}

/// How we access a field in the generated code.
#[derive(Clone, Copy)]
enum FieldAccess {
    /// `self.field_name`
    SelfDot,
    /// bare binding (used inside enum match arms)
    #[allow(dead_code)]
    Binding,
}

/// Encode a set of named fields as a map, sorted by wire name.
fn encode_named_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    access: FieldAccess,
) -> syn::Result<TokenStream> {
    let mut entries: Vec<(String, &syn::Ident, FieldAttrs)> = Vec::new();
    for f in fields {
        let ident = f.ident.as_ref().unwrap();
        let attrs = FieldAttrs::from_ast(&f.attrs)?;
        let wire = attrs.wire_name(ident);
        entries.push((wire, ident, attrs));
    }
    // BTreeMap-order: sort by wire name.
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let len = entries.len() as u64;
    let encodes = entries.iter().map(|(wire, ident, attrs)| {
        let key_encode = quote! { __enc.push_map_k(#wire)?; };

        let value_encode = match access {
            FieldAccess::SelfDot => {
                if let Some(func) = &attrs.serialize_with {
                    quote! { #func(&self.#ident, __enc)?; }
                } else {
                    quote! { genlayer_calldata::codec::Encode::encode(&self.#ident, __enc)?; }
                }
            }
            FieldAccess::Binding => {
                if let Some(func) = &attrs.serialize_with {
                    quote! { #func(#ident, __enc)?; }
                } else {
                    quote! { genlayer_calldata::codec::Encode::encode(#ident, __enc)?; }
                }
            }
        };

        quote! {
            #key_encode
            #value_encode
        }
    });

    Ok(quote! {
        __enc.start_map(#len)?;
        #(#encodes)*
        Ok(())
    })
}

// ── Enums ────────────────────────────────────────────────────────────

fn encode_enum(data: &syn::DataEnum, container: &ContainerAttrs) -> syn::Result<TokenStream> {
    if let Some(tag_field) = &container.tag {
        encode_enum_tagged(data, tag_field)
    } else {
        encode_enum_external(data)
    }
}

/// Externally tagged: `"Variant"` or `{"Variant": <payload>}`.
fn encode_enum_external(data: &syn::DataEnum) -> syn::Result<TokenStream> {
    let arms = data
        .variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            let wire = vattrs.variant_wire_name(variant_ident);

            match &v.fields {
                Fields::Unit => Ok(quote! {
                    Self::#variant_ident => {
                        __enc.push_str(#wire)?;
                        Ok(())
                    }
                }),

                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    let fattrs = FieldAttrs::from_ast(&fields.unnamed[0].attrs)?;
                    let inner_encode = if let Some(func) = &fattrs.serialize_with {
                        quote! { #func(__inner, __enc)?; }
                    } else {
                        quote! { genlayer_calldata::codec::Encode::encode(__inner, __enc)?; }
                    };
                    Ok(quote! {
                        Self::#variant_ident(__inner) => {
                            __enc.start_map(1)?;
                            __enc.push_map_k(#wire)?;
                            #inner_encode
                            Ok(())
                        }
                    })
                }

                Fields::Unnamed(fields) => {
                    let n = fields.unnamed.len();
                    let bindings: Vec<_> = (0..n)
                        .map(|i| {
                            syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site())
                        })
                        .collect();
                    let field_encodes = fields
                        .unnamed
                        .iter()
                        .zip(&bindings)
                        .map(|(f, b)| {
                            let fattrs = FieldAttrs::from_ast(&f.attrs)?;
                            if let Some(func) = &fattrs.serialize_with {
                                Ok(quote! { #func(#b, __enc)?; })
                            } else {
                                Ok(quote! { genlayer_calldata::codec::Encode::encode(#b, __enc)?; })
                            }
                        })
                        .collect::<syn::Result<Vec<_>>>()?;
                    let len = n as u64;
                    Ok(quote! {
                        Self::#variant_ident(#(#bindings),*) => {
                            __enc.start_map(1)?;
                            __enc.push_map_k(#wire)?;
                            __enc.start_array(#len)?;
                            #(#field_encodes)*
                            Ok(())
                        }
                    })
                }

                Fields::Named(fields) => {
                    // Collect field attrs and sort by wire name.
                    let mut entries: Vec<(String, &syn::Ident, FieldAttrs)> = Vec::new();
                    for f in &fields.named {
                        let ident = f.ident.as_ref().unwrap();
                        let attrs = FieldAttrs::from_ast(&f.attrs)?;
                        let w = attrs.wire_name(ident);
                        entries.push((w, ident, attrs));
                    }
                    entries.sort_by(|a, b| a.0.cmp(&b.0));

                    let inner_len = entries.len() as u64;
                    let all_idents: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();

                    let field_encodes = entries.iter().map(|(w, ident, attrs)| {
                        let val = if let Some(func) = &attrs.serialize_with {
                            quote! { #func(#ident, __enc)?; }
                        } else {
                            quote! { genlayer_calldata::codec::Encode::encode(#ident, __enc)?; }
                        };
                        quote! {
                            __enc.push_map_k(#w)?;
                            #val
                        }
                    });

                    Ok(quote! {
                        Self::#variant_ident { #(#all_idents),* } => {
                            __enc.start_map(1)?;
                            __enc.push_map_k(#wire)?;
                            __enc.start_map(#inner_len)?;
                            #(#field_encodes)*
                            Ok(())
                        }
                    })
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        match self {
            #(#arms)*
        }
    })
}

/// Internally tagged: `{"tag": "Variant", ...fields...}`.
fn encode_enum_tagged(data: &syn::DataEnum, tag_field: &str) -> syn::Result<TokenStream> {
    let arms = data
        .variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            let wire = vattrs.variant_wire_name(variant_ident);

            match &v.fields {
                Fields::Unit => {
                    // {"tag": "Variant"}
                    Ok(quote! {
                        Self::#variant_ident => {
                            __enc.start_map(1)?;
                            __enc.push_map_k(#tag_field)?;
                            __enc.push_str(#wire)?;
                            Ok(())
                        }
                    })
                }

                Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                    variant_ident,
                    "internally tagged enums do not support tuple variants",
                )),

                Fields::Named(fields) => {
                    // {"tag": "Variant", ...sorted fields...}
                    let mut entries: Vec<(String, &syn::Ident, FieldAttrs)> = Vec::new();
                    for f in &fields.named {
                        let ident = f.ident.as_ref().unwrap();
                        let attrs = FieldAttrs::from_ast(&f.attrs)?;
                        let w = attrs.wire_name(ident);
                        if w == tag_field {
                            return Err(syn::Error::new_spanned(
                                ident,
                                format!("field name conflicts with tag field `{tag_field}`"),
                            ));
                        }
                        entries.push((w, ident, attrs));
                    }

                    // Tag + fields, all sorted together.
                    // Insert the tag entry and sort.
                    let total = (entries.len() + 1) as u64;

                    // We need to interleave tag among fields in sorted order.
                    entries.sort_by(|a, b| a.0.cmp(&b.0));

                    // Find insert position for the tag key.
                    let tag_pos = entries
                        .iter()
                        .position(|(w, _, _)| w.as_str() > tag_field)
                        .unwrap_or(entries.len());

                    let all_idents: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();

                    // Build encoding statements in sorted key order.
                    let mut stmts = Vec::new();
                    for (i, (w, ident, attrs)) in entries.iter().enumerate() {
                        if i == tag_pos {
                            stmts.push(quote! {
                                __enc.push_map_k(#tag_field)?;
                                __enc.push_str(#wire)?;
                            });
                        }
                        let val = if let Some(func) = &attrs.serialize_with {
                            quote! { #func(#ident, __enc)?; }
                        } else {
                            quote! { genlayer_calldata::codec::Encode::encode(#ident, __enc)?; }
                        };
                        stmts.push(quote! {
                            __enc.push_map_k(#w)?;
                            #val
                        });
                    }
                    // If tag goes at the end.
                    if tag_pos == entries.len() {
                        stmts.push(quote! {
                            __enc.push_map_k(#tag_field)?;
                            __enc.push_str(#wire)?;
                        });
                    }

                    Ok(quote! {
                        Self::#variant_ident { #(#all_idents),* } => {
                            __enc.start_map(#total)?;
                            #(#stmts)*
                            Ok(())
                        }
                    })
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        match self {
            #(#arms)*
        }
    })
}
