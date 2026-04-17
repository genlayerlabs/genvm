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
        impl #impl_generics genlayer_calldata::codec::Decode for #name #ty_generics #where_clause {
            fn decode<__D: genlayer_calldata::codec::Deserializer>(
                __deserializer: __D,
            ) -> ::core::result::Result<Self, genlayer_calldata::codec::Error> {
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
                Ok(quote! {
                    <#ty as genlayer_calldata::codec::Decode>::decode(__deserializer)
                        .map(#name)
                })
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
                        ) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
                            #(
                                let #vars = __seq.next_element::<#field_tys>()?
                                    .ok_or(genlayer_calldata::codec::Error::Custom(
                                        ::std::format!("expected {} tuple elements", #len)
                                    ))?;
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
                fn visit_null(self) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
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
                            genlayer_calldata::codec::Error::FieldMissing(#wire)
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
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
                #(
                    let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                )*
                while let ::core::option::Option::Some((__key, __val)) =
                    __map.next_element::<genlayer_calldata::Value>()?
                {
                    match __key {
                        #(#match_arms)*
                        _ => {}
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
    if let Some(tag_field) = &container.tag {
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

    // Non-unit variant arms for visit_map (single-key map)
    let map_arms: Vec<_> = data
        .variants
        .iter()
        .filter(|v| !matches!(v.fields, Fields::Unit))
        .map(|v| {
            let vattrs = FieldAttrs::from_ast(&v.attrs)?;
            let wire = vattrs.variant_wire_name(&v.ident);
            let ident = &v.ident;
            let decode_body = decode_variant_payload(name, ident, &v.fields)?;
            Ok(quote! { #wire => { #decode_body } })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let has_map = !map_arms.is_empty();

    let visit_str_impl = if has_unit {
        quote! {
            fn visit_str(self, __val: &str) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
                match __val {
                    #(#unit_arms)*
                    _ => ::core::result::Result::Err(genlayer_calldata::codec::Error::Custom(
                        ::std::format!("unknown variant `{}`, expected one of: {}", __val, #names_joined)
                    )),
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
                _len: u64,
                mut __map: __A,
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
                let (__key, __val) = __map
                    .next_element::<genlayer_calldata::Value>()?
                    .ok_or(genlayer_calldata::codec::Error::Custom(
                        "expected single-key map for enum variant".into(),
                    ))?;
                match __key {
                    #(#map_arms)*
                    _ => ::core::result::Result::Err(genlayer_calldata::codec::Error::Custom(
                        ::std::format!("unknown variant `{}`, expected one of: {}", __key, #names_joined)
                    )),
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

/// Decode the payload for a non-unit enum variant from a `Value`.
fn decode_variant_payload(
    enum_name: &syn::Ident,
    variant_ident: &syn::Ident,
    fields: &Fields,
) -> syn::Result<TokenStream> {
    match fields {
        Fields::Unit => unreachable!(),

        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let ty = &fields.unnamed[0].ty;
            Ok(quote! {
                let __inner = <#ty as genlayer_calldata::codec::Decode>::decode(
                    genlayer_calldata::codec::ValueDeserializer(__val)
                )?;
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
                let genlayer_calldata::Value::Array(__arr) = __val else {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::Error::Unexpected("expected array for tuple variant")
                    );
                };
                if __arr.len() != #n {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::Error::Custom(
                            ::std::format!("expected {} elements, got {}", #n, __arr.len())
                        )
                    );
                }
                let mut __iter = __arr.into_iter();
                #(
                    let #vars = <#tys as genlayer_calldata::codec::Decode>::decode(
                        genlayer_calldata::codec::ValueDeserializer(__iter.next().unwrap())
                    )?;
                )*
                ::core::result::Result::Ok(#enum_name::#variant_ident(#(#vars),*))
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
                                    genlayer_calldata::codec::Error::FieldMissing(#wire)
                                )?
                            }
                        }
                    });

            Ok(quote! {
                let genlayer_calldata::Value::Map(__inner_map) = __val else {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::Error::Unexpected("expected map for struct variant")
                    );
                };
                #(
                    let mut #option_vars: ::core::option::Option<#field_tys> = ::core::option::Option::None;
                )*
                for (__k, __v) in __inner_map {
                    match __k.as_str() {
                        #(#match_arms)*
                        _ => {}
                    }
                }
                ::core::result::Result::Ok(#enum_name::#variant_ident {
                    #(#field_constructions),*
                })
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
                                        genlayer_calldata::codec::Error::FieldMissing(#w)
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
                                    _ => {}
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
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::Error> {
                // Collect all entries; extract tag.
                let mut __entries = ::std::collections::BTreeMap::<::std::string::String, genlayer_calldata::Value>::new();
                while let ::core::option::Option::Some((__key, __val)) =
                    __map.next_element::<genlayer_calldata::Value>()?
                {
                    __entries.insert(__key.to_owned(), __val);
                }

                let __tag_val = __entries
                    .remove(#tag_field)
                    .ok_or(genlayer_calldata::codec::Error::FieldMissing(#tag_field))?;

                let genlayer_calldata::Value::Str(__tag_str) = __tag_val else {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::Error::Unexpected("expected string for tag field")
                    );
                };

                match __tag_str.as_str() {
                    #(#variant_arms)*
                    _ => ::core::result::Result::Err(genlayer_calldata::codec::Error::Custom(
                        ::std::format!("unknown variant `{}`, expected one of: {}", __tag_str, #names_joined)
                    )),
                }
            }
        }
        __deserializer.deserialize(__V)
    })
}
