//! Decode derivation for internally tagged enums (`#[calldata(tag = "...")]`).
//!
//! Wire representation: a single map mixing the tag entry (`"<tag>": "Variant"`)
//! with the chosen variant's fields, e.g. `{"type": "Beta", "val": true}`.
//!
//! The map keys arrive strictly sorted (see the wire format), so the tag entry
//! is not guaranteed to come first — historically that forced buffering the whole
//! map into a [`Value`] before the variant could be resolved. Instead we decode
//! in a single streaming pass:
//!
//! 1. accumulate, at macro-expansion time, the *union* of every variant's fields
//!    (each wire name mapped to one slot) plus the min/max number of map entries
//!    any variant can have;
//! 2. at run time, first check the map length is within `[min, max]`;
//! 3. then walk the entries: each key is matched against the tag and the union of
//!    known fields (unknown → error), and its value is decoded into the slot;
//! 4. finally dispatch on the tag string to assemble the concrete variant.
//!
//! A field name shared by several variants collapses to one union slot. When all
//! those variants give it the same type and `deserialize_with`, the slot is
//! *monomorphic* and the value is decoded straight into that type — no
//! intermediate [`Value`]. When they disagree, the type is not knowable until the
//! tag is seen, so the slot is *ambiguous*: it is decoded into a [`Value`] and
//! converted to the exact per-variant type during assembly. Ambiguous slots emit
//! a compile-time warning since they reintroduce the intermediate `Value`.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Fields, Type};

use crate::attrs::FieldAttrs;

struct FieldInfo<'a> {
    wire: String,
    ident: &'a syn::Ident,
    ty: &'a Type,
    attrs: FieldAttrs,
    /// Index into the field union shared across all variants.
    union_idx: usize,
}

enum VariantKind<'a> {
    Unit,
    Named(Vec<FieldInfo<'a>>),
}

struct VariantInfo<'a> {
    wire: String,
    ident: &'a syn::Ident,
    kind: VariantKind<'a>,
}

/// One slot in the union of all variants' fields. Variants sharing a wire name
/// collapse to a single slot.
struct UnionField {
    wire: String,
    /// `Some` when every variant using this name agrees on the decode strategy
    /// (same type and `deserialize_with`) — decoded eagerly into that type.
    /// `None` when they disagree — decoded into a `Value` and converted to the
    /// exact type per-variant during assembly.
    mono: Option<MonoDecode>,
    /// Decode signature `(type, deserialize_with)` of the first usage, used to
    /// detect disagreement while building the union.
    sig: (String, Option<String>),
}

struct MonoDecode {
    ty_tokens: TokenStream,
    de_with: Option<syn::Path>,
}

fn decode_signature(
    ty_tokens: &TokenStream,
    de_with: &Option<syn::Path>,
) -> (String, Option<String>) {
    (
        ty_tokens.to_string(),
        de_with.as_ref().map(|p| p.to_token_stream().to_string()),
    )
}

/// Internally tagged: `{"tag": "Variant", ...fields...}`.
pub fn decode(
    name: &syn::Ident,
    data: &syn::DataEnum,
    tag_field: &str,
) -> syn::Result<TokenStream> {
    // ── Collect variants and accumulate the field union ──────────────
    let mut union: Vec<UnionField> = Vec::new();
    let mut variants: Vec<VariantInfo> = Vec::new();

    for v in &data.variants {
        let vattrs = FieldAttrs::from_ast(&v.attrs)?;
        let wire = vattrs.variant_wire_name(&v.ident);

        let kind = match &v.fields {
            Fields::Unit => VariantKind::Unit,
            Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    "internally tagged enums do not support tuple variants",
                ));
            }
            Fields::Named(fields) => {
                let mut infos: Vec<FieldInfo> = Vec::new();
                for f in &fields.named {
                    let ident = f.ident.as_ref().unwrap();
                    let attrs = FieldAttrs::from_ast(&f.attrs)?;
                    let fwire = attrs.wire_name(ident);
                    let ty_tokens = f.ty.to_token_stream();

                    if fwire == tag_field {
                        return Err(syn::Error::new_spanned(
                            ident,
                            "field name collides with the enum tag field",
                        ));
                    }

                    // Two fields of the same variant resolving to the same wire
                    // name would share a `union_idx` and overwrite each other's
                    // slot; reject it instead of silently miscompiling.
                    if infos.iter().any(|i| i.wire == fwire) {
                        return Err(syn::Error::new_spanned(
                            ident,
                            format!("duplicate wire name `{fwire}` within the variant"),
                        ));
                    }

                    let sig = decode_signature(&ty_tokens, &attrs.deserialize_with);

                    // Merge into the union. A second usage that disagrees on the
                    // decode signature turns the slot ambiguous (`mono = None`).
                    let union_idx = match union.iter().position(|u| u.wire == fwire) {
                        Some(idx) => {
                            if union[idx].sig != sig {
                                union[idx].mono = None;
                            }
                            idx
                        }
                        None => {
                            union.push(UnionField {
                                wire: fwire.clone(),
                                mono: Some(MonoDecode {
                                    ty_tokens: ty_tokens.clone(),
                                    de_with: attrs.deserialize_with.clone(),
                                }),
                                sig,
                            });
                            union.len() - 1
                        }
                    };

                    infos.push(FieldInfo {
                        wire: fwire,
                        ident,
                        ty: &f.ty,
                        attrs,
                        union_idx,
                    });
                }
                VariantKind::Named(infos)
            }
        };

        variants.push(VariantInfo {
            wire,
            ident: &v.ident,
            kind,
        });
    }

    // ── Ambiguous-field warnings ─────────────────────────────────────
    // Each ambiguous slot reintroduces an intermediate `Value`; warn via the
    // standard stable-Rust deprecation trick (zero runtime cost).
    let warnings = union.iter().filter(|u| u.mono.is_none()).map(|u| {
        let msg = format!(
            "internally tagged enum `{name}`: field `{}` has different types across \
             variants, so it is buffered as an intermediate `Maybe<Value>` before \
             being converted to the per-variant type",
            u.wire,
        );
        quote! {
            const _: () = {
                #[deprecated(note = #msg)]
                const AMBIGUOUS_FIELD: () = ();
                AMBIGUOUS_FIELD
            };
        }
    });

    // ── Min/max number of map entries across all variants ────────────
    // Every variant carries the tag entry (+1). A named variant additionally
    // requires its non-defaulted fields (min) and admits all of them (max).
    let mut min_len = usize::MAX;
    let mut max_len = 0usize;
    for v in &variants {
        let (vmin, vmax) = match &v.kind {
            VariantKind::Unit => (1, 1),
            VariantKind::Named(fields) => {
                let required = fields.iter().filter(|f| f.attrs.default.is_none()).count();
                (1 + required, 1 + fields.len())
            }
        };
        min_len = min_len.min(vmin);
        max_len = max_len.max(vmax);
    }
    let min_len = proc_macro2::Literal::usize_unsuffixed(min_len);
    let max_len = proc_macro2::Literal::usize_unsuffixed(max_len);

    // ── Per-union-field slots + streaming key dispatch ───────────────
    let slot_vars: Vec<syn::Ident> = (0..union.len())
        .map(|i| syn::Ident::new(&format!("__u{i}"), proc_macro2::Span::call_site()))
        .collect();
    // Ambiguous slots buffer the value lazily: a byte-backed deserializer keeps
    // the raw wire bytes (and only validates), so the eventual per-variant decode
    // reads straight from those bytes — no `Value` is ever built.
    let deferred_ty = quote! {
        genlayer_calldata::codec::Maybe<genlayer_calldata::Value>
    };
    let slot_tys: Vec<TokenStream> = union
        .iter()
        .map(|u| match &u.mono {
            Some(m) => m.ty_tokens.clone(),
            None => deferred_ty.clone(),
        })
        .collect();

    // The tag is index 0; union fields are indices 1..=n.
    let mut key_arms: Vec<TokenStream> = Vec::new();
    let mut assign_arms: Vec<TokenStream> = Vec::new();
    for (i, (u, slot)) in union.iter().zip(&slot_vars).enumerate() {
        let wire = &u.wire;
        let idx = proc_macro2::Literal::usize_unsuffixed(i + 1);
        key_arms.push(quote! { #wire => #idx, });

        // What goes into the slot for this entry.
        let read = match &u.mono {
            // Ambiguous → defer into a `Maybe<Value>` (byte-backed sources keep
            // the raw bytes instead of materializing a `Value`).
            None => quote! {
                __map.next_value::<genlayer_calldata::codec::Maybe<genlayer_calldata::Value>>()?
            },
            // Monomorphic with `deserialize_with` → `Value` then the function.
            Some(MonoDecode {
                de_with: Some(func),
                ..
            }) => quote! {
                {
                    let __v = __map.next_value::<genlayer_calldata::Value>()?;
                    #func(__v)?
                }
            },
            // Monomorphic plain → decode straight into the typed slot.
            Some(MonoDecode { ty_tokens, .. }) => {
                quote! { __map.next_value::<#ty_tokens>()? }
            }
        };

        assign_arms.push(quote! {
            #idx => {
                if #slot.is_some() {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::DuplicateField(#wire)
                    );
                }
                #slot = ::core::option::Option::Some(#read);
            }
        });
    }

    // ── Tag dispatch → variant assembly ──────────────────────────────
    let all_names: Vec<&str> = variants.iter().map(|v| v.wire.as_str()).collect();
    let names_joined = all_names.join(", ");

    // Reject any filled slot that does not belong to the selected variant. For a
    // unit variant `used` is empty, so every union slot is foreign and must be
    // absent — otherwise `{"radius": 1, "type": "Empty"}` would silently decode
    // as `Empty`, ignoring the foreign `radius`.
    let foreign_checks = |used: &std::collections::HashSet<usize>| {
        union
            .iter()
            .zip(&slot_vars)
            .enumerate()
            .filter_map(|(i, (u, slot))| {
                if used.contains(&i) {
                    return None;
                }
                let fwire = &u.wire;
                Some(quote! {
                    if #slot.is_some() {
                        return ::core::result::Result::Err(
                            genlayer_calldata::codec::DecodeError::UnknownField(
                                #fwire.to_owned()
                            )
                        );
                    }
                })
            })
            .collect::<Vec<_>>()
    };

    let variant_arms = variants.iter().map(|v| {
        let wire = &v.wire;
        let ident = v.ident;
        match &v.kind {
            VariantKind::Unit => {
                let checks = foreign_checks(&std::collections::HashSet::new());
                quote! {
                    #wire => {
                        #(#checks)*
                        ::core::result::Result::Ok(#name::#ident)
                    }
                }
            }
            VariantKind::Named(fields) => {
                let used: std::collections::HashSet<usize> =
                    fields.iter().map(|f| f.union_idx).collect();

                // Reject fields belonging to other variants but not this one.
                let foreign_checks = foreign_checks(&used);

                let constructions = fields.iter().map(|f| {
                    let fi = f.ident;
                    let slot = &slot_vars[f.union_idx];
                    let fwire = &f.wire;
                    let is_mono = union[f.union_idx].mono.is_some();

                    // For an ambiguous slot the buffered `Maybe<Value>` must be
                    // converted to this variant's exact type. With `deserialize_with`
                    // the function needs a `Value`, so materialize first; otherwise
                    // `decode_into` reads straight into the target type (no `Value`
                    // on the byte-backed path).
                    let convert = |val: TokenStream| {
                        if is_mono {
                            val
                        } else if let Some(func) = &f.attrs.deserialize_with {
                            quote! {
                                {
                                    let __mv = #val.materialize()?;
                                    #func(__mv)?
                                }
                            }
                        } else {
                            let ty = f.ty;
                            quote! { #val.decode_into::<#ty>()? }
                        }
                    };

                    if let Some(default_fn) = &f.attrs.default {
                        let converted = convert(quote! { __v });
                        quote! {
                            #fi: match #slot {
                                ::core::option::Option::Some(__v) => #converted,
                                ::core::option::Option::None => #default_fn(),
                            }
                        }
                    } else {
                        let take = quote! {
                            #slot.ok_or(
                                genlayer_calldata::codec::DecodeError::FieldMissing(#fwire)
                            )?
                        };
                        if is_mono {
                            quote! { #fi: #take }
                        } else {
                            let converted = convert(quote! { __v });
                            quote! {
                                #fi: {
                                    let __v = #take;
                                    #converted
                                }
                            }
                        }
                    }
                });

                quote! {
                    #wire => {
                        #(#foreign_checks)*
                        ::core::result::Result::Ok(#name::#ident {
                            #(#constructions),*
                        })
                    }
                }
            }
        }
    });

    Ok(quote! {
        #(#warnings)*

        struct __V;
        impl genlayer_calldata::codec::Visitor for __V {
            type Value = #name;
            fn visit_map<__A: genlayer_calldata::codec::MapAccess>(
                self,
                __len: u64,
                mut __map: __A,
            ) -> ::core::result::Result<#name, genlayer_calldata::codec::DecodeError> {
                // (a) the map length must be in range for some variant.
                if __len < #min_len as u64 {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::LengthMismatch {
                            expected: #min_len,
                            got: __len as usize,
                        },
                    );
                }
                if __len > #max_len as u64 {
                    return ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::LengthMismatch {
                            expected: #max_len,
                            got: __len as usize,
                        },
                    );
                }

                let mut __tag: ::core::option::Option<::std::string::String> =
                    ::core::option::Option::None;
                #(
                    let mut #slot_vars: ::core::option::Option<#slot_tys> =
                        ::core::option::Option::None;
                )*

                // (b) walk entries: known key -> slot, unknown key -> fail.
                while let ::core::option::Option::Some(__key) = __map.next_key()? {
                    let __idx: usize = match __key {
                        #tag_field => 0,
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
                        0 => {
                            if __tag.is_some() {
                                return ::core::result::Result::Err(
                                    genlayer_calldata::codec::DecodeError::DuplicateField(#tag_field)
                                );
                            }
                            __tag = ::core::option::Option::Some(__map.next_value::<::std::string::String>()?);
                        }
                        #(#assign_arms)*
                        _ => ::core::unreachable!(),
                    }
                }

                let __tag = __tag.ok_or(
                    genlayer_calldata::codec::DecodeError::FieldMissing(#tag_field)
                )?;

                match __tag.as_str() {
                    #(#variant_arms)*
                    _ => ::core::result::Result::Err(
                        genlayer_calldata::codec::DecodeError::UnknownVariant {
                            got: __tag,
                            expected: #names_joined,
                        }
                    ),
                }
            }
        }
        __deserializer.deserialize(__V)
    })
}
