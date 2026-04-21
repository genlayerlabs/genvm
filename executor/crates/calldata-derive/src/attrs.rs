use syn::Lit;

/// Container-level attributes (`#[calldata(...)]` on struct/enum).
pub struct ContainerAttrs {
    /// For enums: internally tagged with this field name.
    /// `#[calldata(tag = "type")]`
    pub tag: Option<String>,
    /// For enums: untagged representation.
    /// `#[calldata(untagged)]`
    pub untagged: bool,
}

/// Field/variant-level attributes (`#[calldata(...)]` on fields or enum variants).
pub struct FieldAttrs {
    /// `#[calldata(rename = "wire_name")]`
    pub rename: Option<String>,
    /// `#[calldata(serialize_with = path::to::fn)]`
    pub serialize_with: Option<syn::Path>,
    /// `#[calldata(deserialize_with = path::to::fn)]`
    pub deserialize_with: Option<syn::Path>,
    /// `#[calldata(default = path::to::fn)]`
    pub default: Option<syn::Path>,
}

impl ContainerAttrs {
    pub fn from_ast(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut tag = None;
        let mut untagged = false;

        for attr in attrs {
            if !attr.path().is_ident("calldata") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("tag") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        tag = Some(s.value());
                    } else {
                        return Err(meta.error("expected string literal for `tag`"));
                    }
                    return Ok(());
                }
                if meta.path.is_ident("untagged") {
                    untagged = true;
                    return Ok(());
                }
                Err(meta.error("unknown calldata container attribute"))
            })?;
        }

        if tag.is_some() && untagged {
            return Err(syn::Error::new_spanned(
                &attrs[0],
                "`tag` and `untagged` are mutually exclusive",
            ));
        }

        Ok(ContainerAttrs { tag, untagged })
    }
}

impl FieldAttrs {
    pub fn from_ast(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut rename = None;
        let mut serialize_with = None;
        let mut deserialize_with = None;
        let mut default = None;

        for attr in attrs {
            if !attr.path().is_ident("calldata") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        rename = Some(s.value());
                    } else {
                        return Err(meta.error("expected string literal for `rename`"));
                    }
                    return Ok(());
                }
                if meta.path.is_ident("serialize_with") {
                    let value = meta.value()?;
                    serialize_with = Some(value.parse::<syn::Path>()?);
                    return Ok(());
                }
                if meta.path.is_ident("deserialize_with") {
                    let value = meta.value()?;
                    deserialize_with = Some(value.parse::<syn::Path>()?);
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    let value = meta.value()?;
                    default = Some(value.parse::<syn::Path>()?);
                    return Ok(());
                }
                Err(meta.error("unknown calldata field attribute"))
            })?;
        }

        Ok(FieldAttrs {
            rename,
            serialize_with,
            deserialize_with,
            default,
        })
    }

    /// The wire name for a struct field.
    pub fn wire_name(&self, ident: &syn::Ident) -> String {
        self.rename.clone().unwrap_or_else(|| ident.to_string())
    }

    /// The wire name for an enum variant.
    pub fn variant_wire_name(&self, ident: &syn::Ident) -> String {
        self.rename.clone().unwrap_or_else(|| ident.to_string())
    }
}
