use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{ContainerAttrs, FieldAttrs};

pub fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let container = ContainerAttrs::from_ast(&input.attrs)?;

    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(tp) = param {
            tp.bounds
                .push(syn::parse_quote!(genlayer_calldata::codec::Decode));
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => decode_struct(name, &data.fields)?,
        Data::Enum(data) => decode_enum(name, data, &container)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Decode cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        #[allow(deprecated)]
        impl #impl_generics genlayer_calldata::codec::Decode for #name #ty_generics #where_clause {
            fn decode<__D: genlayer_calldata::codec::Deserializer>(
                __deserializer: __D,
            ) -> ::core::result::Result<Self, genlayer_calldata::codec::DecodeError> {
                #body
            }
        }
    })
}

// ── Structs ──────────────────────────────────────────────────────────

fn decode_struct(name: &syn::Ident, fields: &Fields) -> syn::Result<TokenStream> {
    match fields {
        Fields::Named(fields) => decode_named_fields(name, &fields.named),
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                let ty = &fields.unnamed[0].ty;
                let attrs = FieldAttrs::from_ast(&fields.unnamed[0].attrs)?;
                if let Some(func) = &attrs.deserialize_with {
                    Ok(quote! {
                        let __val = <genlayer_calldata::Value as genlayer_calldata::codec::Decode>::decode(__deserializer)?;
                        #func(__val).map(#name)
                    })
                } else {
                    Ok(quote! {
                        <#ty as genlayer_calldata::codec::Decode>::decode(__deserializer)
                            .map(#name)
                    })
                }
            } else {
                let len = fields.unnamed.len();
                let field_tys: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
                let vars: Vec<_> = (0..len)
                    .map(|i| syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site()))
                    .collect();
                Ok(quote! {
                    struct __V;
                    impl genlayer_calldata::codec::Visitor for __V {
                        type Value = #name;
                        fn visit_seq<__A: genlayer_calldata::codec::SeqAccess>(
                            self,
                            _len: u64,
                            mut __seq: __A,
                        ) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                            if _len != #len as u64 {
                                return ::core::result::Result::Err(
                                    genlayer_calldata::codec::DecodeError::LengthMismatch {
                                        expected: #len,
                                        got: _len as usize,
                                    }
                                );
                            }
                            #(
                                let #vars = __seq.next_element::<#field_tys>()?
                                    .ok_or(genlayer_calldata::codec::DecodeError::LengthMismatch {
                                        expected: #len,
                                        got: 0,
                                    })?;
                            )*
                            Ok(#name(#(#vars),*))
                        }
                    }
                    __deserializer.deserialize(__V)
                })
            }
        }
        Fields::Unit => Ok(quote! {
            struct __V;
            impl genlayer_calldata::codec::Visitor for __V {
                type Value = #name;
                fn visit_null(self) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                    Ok(#name)
                }
            }
            __deserializer.deserialize(__V)
        }),
    }
}

/// Generate decode for named fields (used for structs and enum struct variants).
/// `constructor` is e.g. `quote!(MyStruct)` or `quote!(MyEnum::Variant)`.
fn decode_named_fields(
    name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<TokenStream> {
    let mut entries: Vec<(String, syn::Ident, &syn::Type, FieldAttrs)> = Vec::new();
    for f in fields {
        let ident = f.ident.as_ref().unwrap().clone();
        let attrs = FieldAttrs::from_ast(&f.attrs)?;
        let wire = attrs.wire_name(&ident);
        entries.push((wire, ident, &f.ty, attrs));
    }

    let option_vars: Vec<_> = entries
        .iter()
        .map(|(_, ident, _, _)| {
            syn::Ident::new(&format!("__f_{ident}"), proc_macro2::Span::call_site())
        })
        .collect();

    let match_arms = entries
        .iter()
        .zip(&option_vars)
        .map(|((wire, _, ty, attrs), var)| {
            let decode_expr = if let Some(func) = &attrs.deserialize_with {
                quote! { #func(__val)? }
            } else {
                quote! {
                    <#ty as genlayer_calldata::codec::Decode>::decode(
                        genlayer_calldata::codec::ValueDeserializer(__val)
                    )?
                }
            };
            quote! { #wire => { #var = ::core::option::Option::Some(#decode_expr); } }
        });

    let field_constructions =
        entries
            .iter()
            .zip(&option_vars)
            .map(|((wire, ident, _, attrs), var)| {
                if let Some(default_fn) = &attrs.default {
                    quote! { #ident: #var.unwrap_or_else(#default_fn) }
                } else {
                    quote! {
                        #ident: #var.ok_or(
                            genlayer_calldata::codec::DecodeError::FieldMissing(#wire)
                        )?
                    }
                }
            });

    let field_idents: Vec<_> = entries.iter().map(|(_, ident, _, _)| ident).collect();
    let field_tys: Vec<_> = entries.iter().map(|(_, _, ty, _)| *ty).collect();
    let _ = &field_idents; // suppress unused

    Ok(quote! {
        struct __V;
        impl genlayer_calldata::codec::Visitor for __V {
            type Value = #name;
            fn visit_map<__A: genlayer_calldata::codec::MapAccess>(
                self,
                _len: u64,
                mut __map: __A,
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                #(
                    let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                )*
                while let ::core::option::Option::Some((__key, __val)) =
                    __map.next_element::<genlayer_calldata::Value>()?
                {
                    match __key {
                        #(#match_arms)*
                        __other => {
                            return ::core::result::Result::Err(
                                genlayer_calldata::codec::DecodeError::UnknownField(
                                    __other.to_owned()
                                )
                            );
                        }
                    }
                }
                Ok(#name {
                    #(#field_constructions),*
                })
            }
        }
        __deserializer.deserialize(__V)
    })
}

// ── Enums ────────────────────────────────────────────────────────────

fn decode_enum(
    name: &syn::Ident,
    data: &syn::DataEnum,
    container: &ContainerAttrs,
) -> syn::Result<TokenStream> {
    if container.untagged {
        decode_enum_untagged(name, data)
    } else if let Some(tag_field) = &container.tag {
        decode_enum_tagged(name, data, tag_field)
    } else {
        decode_enum_external(name, data)
    }
}

fn variant_names_list(data: &syn::DataEnum) -> Vec<String> {
    data.variants
        .iter()
        .map(|v| {
            let attrs = FieldAttrs::from_ast(&v.attrs).unwrap();
            attrs.variant_wire_name(&v.ident)
        })
        .collect()
}

/// Untagged: try each variant in order; first successful decode wins.
fn decode_enum_untagged(name: &syn::Ident, data: &syn::DataEnum) -> syn::Result<TokenStream> {
    let variant_attempts: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            decode_untagged_variant_attempt(name, variant_ident, &v.fields, &vattrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let all_names = variant_names_list(data);
    let names_joined = all_names.join(", ");

    Ok(quote! {
        // Decode into a Value first, then try each variant.
        let __val = <genlayer_calldata::Value as genlayer_calldata::codec::Decode>::decode(__deserializer)?;
        #(#variant_attempts)*
        ::core::result::Result::Err(genlayer_calldata::codec::DecodeError::UnknownVariant {
            got: ::std::format!("{:?}", __val),
            expected: #names_joined,
        })
    })
}

/// Generate a single attempt to decode a Value into a specific untagged variant.
fn decode_untagged_variant_attempt(
    enum_name: &syn::Ident,
    variant_ident: &syn::Ident,
    fields: &Fields,
    vattrs: &FieldAttrs,
) -> syn::Result<TokenStream> {
    match fields {
        Fields::Unit => Ok(quote! {
            if matches!(&__val, genlayer_calldata::Value::Null) {
                return ::core::result::Result::Ok(#enum_name::#variant_ident);
            }
        }),

        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let ty = &fields.unnamed[0].ty;
            let fattrs = FieldAttrs::from_ast(&fields.unnamed[0].attrs)?;
            let de_fn = fattrs
                .deserialize_with
                .as_ref()
                .or(vattrs.deserialize_with.as_ref());
            let attempt = if let Some(func) = de_fn {
                quote! {
                    if let ::core::result::Result::Ok(__v) = #func(__val.clone()) {
                        return ::core::result::Result::Ok(#enum_name::#variant_ident(__v));
                    }
                }
            } else {
                quote! {
                    if let ::core::result::Result::Ok(__v) =
                        <#ty as genlayer_calldata::codec::Decode>::decode(
                            genlayer_calldata::codec::ValueDeserializer(__val.clone())
                        )
                    {
                        return ::core::result::Result::Ok(#enum_name::#variant_ident(__v));
                    }
                }
            };
            Ok(attempt)
        }

        Fields::Unnamed(fields) => {
            let n = fields.unnamed.len();
            let tys: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
            let vars: Vec<_> = (0..n)
                .map(|i| syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site()))
                .collect();
            Ok(quote! {
                if let ::core::result::Result::Ok(__v) = (|| -> ::core::result::Result<#enum_name, genlayer_calldata::codec::DecodeError> {
                    let genlayer_calldata::Value::Array(__arr) = __val.clone() else {
                        return ::core::result::Result::Err(
                            genlayer_calldata::codec::DecodeError::Unexpected("expected array for tuple variant")
                        );
                    };
                    if __arr.len() != #n {
                        return ::core::result::Result::Err(
                            genlayer_calldata::codec::DecodeError::LengthMismatch {
                                expected: #n,
                                got: __arr.len(),
                            }
                        );
                    }
                    let mut __iter = __arr.into_iter();
                    #(
                        let #vars = <#tys as genlayer_calldata::codec::Decode>::decode(
                            genlayer_calldata::codec::ValueDeserializer(__iter.next().unwrap())
                        )?;
                    )*
                    ::core::result::Result::Ok(#enum_name::#variant_ident(#(#vars),*))
                })() {
                    return ::core::result::Result::Ok(__v);
                }
            })
        }

        Fields::Named(fields) => {
            let mut entries: Vec<(String, &syn::Ident, &syn::Type, FieldAttrs)> = Vec::new();
            for f in &fields.named {
                let ident = f.ident.as_ref().unwrap();
                let attrs = FieldAttrs::from_ast(&f.attrs)?;
                let wire = attrs.wire_name(ident);
                entries.push((wire, ident, &f.ty, attrs));
            }

            let option_vars: Vec<_> = entries
                .iter()
                .map(|(_, ident, _, _)| {
                    syn::Ident::new(&format!("__f_{ident}"), proc_macro2::Span::call_site())
                })
                .collect();

            let field_tys: Vec<_> = entries.iter().map(|(_, _, ty, _)| *ty).collect();

            let match_arms = entries
                .iter()
                .zip(&option_vars)
                .map(|((wire, _, ty, attrs), var)| {
                    let decode_expr = if let Some(func) = &attrs.deserialize_with {
                        quote! { #func(__v)? }
                    } else {
                        quote! {
                            <#ty as genlayer_calldata::codec::Decode>::decode(
                                genlayer_calldata::codec::ValueDeserializer(__v)
                            )?
                        }
                    };
                    quote! { #wire => { #var = ::core::option::Option::Some(#decode_expr); } }
                });

            let field_constructions =
                entries
                    .iter()
                    .zip(&option_vars)
                    .map(|((wire, ident, _, attrs), var)| {
                        if let Some(default_fn) = &attrs.default {
                            quote! { #ident: #var.unwrap_or_else(#default_fn) }
                        } else {
                            quote! {
                                #ident: #var.ok_or(
                                    genlayer_calldata::codec::DecodeError::FieldMissing(#wire)
                                )?
                            }
                        }
                    });

            Ok(quote! {
                if let ::core::result::Result::Ok(__v) = (|| -> ::core::result::Result<#enum_name, genlayer_calldata::codec::DecodeError> {
                    let genlayer_calldata::Value::Map(__inner_map) = __val.clone() else {
                        return ::core::result::Result::Err(
                            genlayer_calldata::codec::DecodeError::Unexpected("expected map for struct variant")
                        );
                    };
                    #(
                        let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                    )*
                    for (__k, __v) in __inner_map {
                        match __k.as_str() {
                            #(#match_arms)*
                            __other => {
                                return ::core::result::Result::Err(
                                    genlayer_calldata::codec::DecodeError::UnknownField(
                                        __other.to_owned()
                                    )
                                );
                            }
                        }
                    }
                    ::core::result::Result::Ok(#enum_name::#variant_ident {
                        #(#field_constructions),*
                    })
                })() {
                    return ::core::result::Result::Ok(__v);
                }
            })
        }
    }
}

/// Externally tagged: `"Variant"` or `{"Variant": <payload>}`.
fn decode_enum_external(name: &syn::Ident, data: &syn::DataEnum) -> syn::Result<TokenStream> {
    let all_names = variant_names_list(data);
    let names_joined = all_names.join(", ");

    // Unit variant arms for visit_str
    let unit_arms: Vec<_> = data
        .variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            let wire = vattrs.variant_wire_name(&v.ident);
            let ident = &v.ident;
            Ok(quote! { #wire => ::core::result::Result::Ok(#name::#ident), })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let has_unit = !unit_arms.is_empty();

    // Non-unit variants are dispatched in two phases: first map the (borrowed)
    // key to an index, then decode the payload directly from the deserializer.
    // The two phases drop the key borrow before the value is read, and decoding
    // the payload via the live deserializer is what lets `Maybe`/`Raw` fields
    // stay deferred instead of being materialized into a `Value` first.
    let nonunit: Vec<&syn::Variant> = data
        .variants
        .iter()
        .filter(|v| !matches!(v.fields, Fields::Unit))
        .collect();

    let mut key_arms: Vec<TokenStream> = Vec::new();
    let mut payload_arms: Vec<TokenStream> = Vec::new();
    for (i, v) in nonunit.iter().enumerate() {
        let vattrs = FieldAttrs::from_ast(&v.attrs)?;
        let wire = vattrs.variant_wire_name(&v.ident);
        let idx = proc_macro2::Literal::usize_unsuffixed(i);
        let payload = decode_variant_payload(name, &v.ident, &v.fields, &vattrs)?;
        key_arms.push(quote! { #wire => #idx, });
        payload_arms.push(quote! { #idx => { #payload } });
    }

    let has_map = !nonunit.is_empty();

    let visit_str_impl = if has_unit {
        quote! {
            fn visit_str(self, __val: &str) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                match __val {
                    #(#unit_arms)*
                    _ => ::core::result::Result::Err(genlayer_calldata::codec::DecodeError::UnknownVariant {
                        got: __val.to_owned(),
                        expected: #names_joined,
                    }),
                }
            }
        }
    } else {
        quote! {}
    };

    let visit_map_impl = if has_map {
        quote! {
            fn visit_map<__A: genlayer_calldata::codec::MapAccess>(
                self,
                __len: u64,
                mut __map: __A,
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                if __len != 1 {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::LengthMismatch {
                            expected: 1,
                            got: __len as usize,
                        },
                    );
                }
                let __idx: usize = match __map.next_key()? {
                    ::core::option::Option::Some(__key) => match __key {
                        #(#key_arms)*
                        _ => {
                            return ::core::result::Result::Err(
                                genlayer_calldata::codec::DecodeError::UnknownVariant {
                                    got: __key.to_owned(),
                                    expected: #names_joined,
                                },
                            );
                        }
                    },
                    ::core::option::Option::None => {
                        return ::core::result::Result::Err(
                            genlayer_calldata::codec::DecodeError::Unexpected(
                                "expected single-key map for enum variant",
                            ),
                        );
                    }
                };
                match __idx {
                    #(#payload_arms)*
                    _ => ::core::unreachable!(),
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        struct __V;
        impl genlayer_calldata::codec::Visitor for __V {
            type Value = #name;
            #visit_str_impl
            #visit_map_impl
        }
        __deserializer.deserialize(__V)
    })
}

/// Decode the payload for a non-unit external-enum variant.
///
/// Generates an expression of type `Result<#enum_name, DecodeError>`, assuming a
/// `MapAccess` named `__map` (positioned on the variant entry's value) is in
/// scope. The payload is read with `__map.next_value` / `next_value_visit` so it
/// decodes straight from the underlying deserializer — keeping `Maybe`/`Raw`
/// fields deferred — instead of materializing a `Value` first.
fn decode_variant_payload(
    enum_name: &syn::Ident,
    variant_ident: &syn::Ident,
    fields: &Fields,
    vattrs: &FieldAttrs,
) -> syn::Result<TokenStream> {
    match fields {
        Fields::Unit => unreachable!(),

        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let ty = &fields.unnamed[0].ty;
            let fattrs = FieldAttrs::from_ast(&fields.unnamed[0].attrs)?;
            let de_fn = fattrs
                .deserialize_with
                .as_ref()
                .or(vattrs.deserialize_with.as_ref());
            let decode_inner = if let Some(func) = de_fn {
                quote! {
                    let __val = __map.next_value::<genlayer_calldata::Value>()?;
                    #func(__val)?
                }
            } else {
                quote! { __map.next_value::<#ty>()? }
            };
            Ok(quote! {
                let __inner = { #decode_inner };
                ::core::result::Result::Ok(#enum_name::#variant_ident(__inner))
            })
        }

        Fields::Unnamed(fields) => {
            let n = fields.unnamed.len();
            let tys: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
            let vars: Vec<_> = (0..n)
                .map(|i| syn::Ident::new(&format!("__f{i}"), proc_macro2::Span::call_site()))
                .collect();
            Ok(quote! {
                struct __PayloadV;
                impl genlayer_calldata::codec::Visitor for __PayloadV {
                    type Value = #enum_name;
                    fn visit_seq<__S: genlayer_calldata::codec::SeqAccess>(
                        self,
                        __len: u64,
                        mut __seq: __S,
                    ) -> ::core::result::Result<#enum_name, genlayer_calldata::codec::DecodeError> {
                        if __len != #n as u64 {
                            return ::core::result::Result::Err(
                                genlayer_calldata::codec::DecodeError::LengthMismatch {
                                    expected: #n,
                                    got: __len as usize,
                                }
                            );
                        }
                        #(
                            let #vars = __seq.next_element::<#tys>()?.ok_or(
                                genlayer_calldata::codec::DecodeError::Unexpected("missing tuple variant element")
                            )?;
                        )*
                        ::core::result::Result::Ok(#enum_name::#variant_ident(#(#vars),*))
                    }
                }
                __map.next_value_visit(__PayloadV)
            })
        }

        Fields::Named(fields) => {
            let mut entries: Vec<(String, &syn::Ident, &syn::Type, FieldAttrs)> = Vec::new();
            for f in &fields.named {
                let ident = f.ident.as_ref().unwrap();
                let attrs = FieldAttrs::from_ast(&f.attrs)?;
                let wire = attrs.wire_name(ident);
                entries.push((wire, ident, &f.ty, attrs));
            }

            let option_vars: Vec<_> = entries
                .iter()
                .map(|(_, ident, _, _)| {
                    syn::Ident::new(&format!("__f_{ident}"), proc_macro2::Span::call_site())
                })
                .collect();

            let field_tys: Vec<_> = entries.iter().map(|(_, _, ty, _)| *ty).collect();

            // Two-phase per-field dispatch (key -> index -> typed value), so the
            // key borrow is released before `next_value` reads the value.
            let mut key_arms: Vec<TokenStream> = Vec::new();
            let mut assign_arms: Vec<TokenStream> = Vec::new();
            for (j, ((_, _, ty, attrs), var)) in entries.iter().zip(&option_vars).enumerate() {
                let wire = &entries[j].0;
                let idx = proc_macro2::Literal::usize_unsuffixed(j);
                key_arms.push(quote! { #wire => #idx, });
                let assign = if let Some(func) = &attrs.deserialize_with {
                    quote! {
                        #idx => {
                            let __v = __m.next_value::<genlayer_calldata::Value>()?;
                            #var = ::core::option::Option::Some(#func(__v)?);
                        }
                    }
                } else {
                    quote! {
                        #idx => { #var = ::core::option::Option::Some(__m.next_value::<#ty>()?); }
                    }
                };
                assign_arms.push(assign);
            }

            let field_constructions =
                entries
                    .iter()
                    .zip(&option_vars)
                    .map(|((wire, ident, _, attrs), var)| {
                        if let Some(default_fn) = &attrs.default {
                            quote! { #ident: #var.unwrap_or_else(#default_fn) }
                        } else {
                            quote! {
                                #ident: #var.ok_or(
                                    genlayer_calldata::codec::DecodeError::FieldMissing(#wire)
                                )?
                            }
                        }
                    });

            Ok(quote! {
                struct __PayloadV;
                impl genlayer_calldata::codec::Visitor for __PayloadV {
                    type Value = #enum_name;
                    fn visit_map<__M: genlayer_calldata::codec::MapAccess>(
                        self,
                        _len: u64,
                        mut __m: __M,
                    ) -> ::core::result::Result<#enum_name, genlayer_calldata::codec::DecodeError> {
                        #(
                            let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                        )*
                        while let ::core::option::Option::Some(__key) = __m.next_key()? {
                            let __idx: usize = match __key {
                                #(#key_arms)*
                                __other => {
                                    return ::core::result::Result::Err(
                                        genlayer_calldata::codec::DecodeError::UnknownField(
                                            __other.to_owned()
                                        )
                                    );
                                }
                            };
                            match __idx {
                                #(#assign_arms)*
                                _ => ::core::unreachable!(),
                            }
                        }
                        ::core::result::Result::Ok(#enum_name::#variant_ident {
                            #(#field_constructions),*
                        })
                    }
                }
                __map.next_value_visit(__PayloadV)
            })
        }
    }
}

/// Internally tagged: `{"tag": "Variant", ...fields...}`.
fn decode_enum_tagged(
    name: &syn::Ident,
    data: &syn::DataEnum,
    tag_field: &str,
) -> syn::Result<TokenStream> {
    let all_names = variant_names_list(data);
    let names_joined = all_names.join(", ");

    let variant_arms: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            let wire = vattrs.variant_wire_name(&v.ident);
            let ident = &v.ident;

            match &v.fields {
                Fields::Unit => Ok(quote! {
                    #wire => ::core::result::Result::Ok(#name::#ident),
                }),

                Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                    ident,
                    "internally tagged enums do not support tuple variants",
                )),

                Fields::Named(fields) => {
                    let mut entries: Vec<(String, &syn::Ident, &syn::Type, FieldAttrs)> =
                        Vec::new();
                    for f in &fields.named {
                        let fi = f.ident.as_ref().unwrap();
                        let attrs = FieldAttrs::from_ast(&f.attrs)?;
                        let w = attrs.wire_name(fi);
                        entries.push((w, fi, &f.ty, attrs));
                    }

                    let option_vars: Vec<_> = entries
                        .iter()
                        .map(|(_, fi, _, _)| {
                            syn::Ident::new(
                                &format!("__f_{fi}"),
                                proc_macro2::Span::call_site(),
                            )
                        })
                        .collect();

                    let field_tys: Vec<_> = entries.iter().map(|(_, _, ty, _)| *ty).collect();

                    let match_arms = entries.iter().zip(&option_vars).map(
                        |((w, _, ty, attrs), var)| {
                            let decode_expr = if let Some(func) = &attrs.deserialize_with {
                                quote! { #func(__v)? }
                            } else {
                                quote! {
                                    <#ty as genlayer_calldata::codec::Decode>::decode(
                                        genlayer_calldata::codec::ValueDeserializer(__v)
                                    )?
                                }
                            };
                            quote! { #w => { #var = ::core::option::Option::Some(#decode_expr); } }
                        },
                    );

                    let field_constructions = entries.iter().zip(&option_vars).map(
                        |((w, fi, _, attrs), var)| {
                            if let Some(default_fn) = &attrs.default {
                                quote! { #fi: #var.unwrap_or_else(#default_fn) }
                            } else {
                                quote! {
                                    #fi: #var.ok_or(
                                        genlayer_calldata::codec::DecodeError::FieldMissing(#w)
                                    )?
                                }
                            }
                        },
                    );

                    Ok(quote! {
                        #wire => {
                            #(
                                let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                            )*
                            for (__k, __v) in __entries {
                                match __k.as_str() {
                                    #(#match_arms)*
                                    __other => {
                                        return ::core::result::Result::Err(
                                            genlayer_calldata::codec::DecodeError::UnknownField(
                                                __other.to_owned()
                                            )
                                        );
                                    }
                                }
                            }
                            ::core::result::Result::Ok(#name::#ident {
                                #(#field_constructions),*
                            })
                        }
                    })
                }
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        struct __V;
        impl genlayer_calldata::codec::Visitor for __V {
            type Value = #name;
            fn visit_map<__A: genlayer_calldata::codec::MapAccess>(
                self,
                _len: u64,
                mut __map: __A,
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                // Collect all entries; extract tag.
                let mut __entries = ::std::collections::BTreeMap::<::std::string::String, genlayer_calldata::Value>::new();
                while let ::core::option::Option::Some((__key, __val)) =
                    __map.next_element::<genlayer_calldata::Value>()?
                {
                    __entries.insert(__key.to_owned(), __val);
                }

                let __tag_val = __entries
                    .remove(#tag_field)
                    .ok_or(genlayer_calldata::codec::DecodeError::FieldMissing(#tag_field))?;

                let genlayer_calldata::Value::Str(__tag_str) = __tag_val else {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::Unexpected("expected string for tag field")
                    );
                };

                match __tag_str.as_str() {
                    #(#variant_arms)*
                    _ => ::core::result::Result::Err(genlayer_calldata::codec::DecodeError::UnknownVariant {
                        got: __tag_str.to_owned(),
                        expected: #names_joined,
                    }),
                }
            }
        }
        __deserializer.deserialize(__V)
    })
}
