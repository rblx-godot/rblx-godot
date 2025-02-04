use proc_macro2::Span;
use syn::{braced, bracketed, parse::{Parse, ParseStream}, punctuated::Punctuated, spanned::Spanned, token::{self, Brace, Bracket, Comma, Eq, Semi, Struct}, Attribute, Error, Field, Generics, Ident, LitBool, LitStr, Path, Result, Signature, Token, Visibility, WhereClause};

#[derive(Debug)]
pub enum SecurityContext {
    None,
    PluginSecurity,
    LocalUserSecurity,
    RobloxScriptSecurity
}

#[derive(Debug)]
pub struct LuaPropertyData {
    pub name: String,
    pub readonly: bool,
    pub get: Option<Path>,
    pub set: Option<Path>,
    pub security_context: SecurityContext,
    pub default: Option<Option<syn::Expr>>,
    pub not_replicated: bool,
    pub transparent: bool,
    pub rust_name: Ident,
    pub span: Span
}

#[derive(Debug)]
pub struct LuaFunctionData {
    pub lua_name: String,
    pub virt: bool,
    pub security_context: SecurityContext,
    pub asyn: bool,

    pub sig: Signature
}

pub fn parse_lua_fn_attr(attr: Attribute) -> Result<LuaFunctionData> {
    let (
        mut lua_name,
        mut virt,
        mut security_context,
        mut asyn,
        mut sig
    ): (
        Option<String>,
        Option<bool>,
        Option<SecurityContext>,
        Option<bool>,
        Option<Signature>
    ) = (
        None,
        None,
        None,
        None,
        None
    );

    attr.parse_nested_meta(|pnm| {
        let ident = match pnm.path.get_ident() {
            Some(s) => s,
            None => {
                return Err(pnm.error("bad option name"))
            }
        };

        let ident = ident.to_string();

        match ident.as_str() {
            "func" => {
                if sig.is_some() { return Err(pnm.error("`func` specified twice or more")) }
                sig = Some(pnm.value()?.parse::<Signature>()?);
            },
            "name" => {
                if lua_name.is_some() { return Err(pnm.error("`name` specified twice or more")) }
                lua_name = Some(pnm.value()?.parse::<LitStr>()?.value());
            },
            "virtual" => {
                if virt.is_some() { return Err(pnm.error("`virtual` specified twice or more")) }
                virt = Some(match pnm.value() { Ok(s) => s.parse::<LitBool>()?.value(), Err(_) => true });
            },
            "async" => {
                if asyn.is_some() { return Err(pnm.error("`async` specified twice or more")) }
                asyn = Some(match pnm.value() { Ok(s) => s.parse::<LitBool>()?.value(), Err(_) => true });
            },
            "security_context" => {
                if security_context.is_some() { return Err(pnm.error("`security_context` specified twice or more")) }
                let value: Ident = pnm.value()?.parse()?;

                let ident = value.to_string();
                match ident.as_str() {
                    "None" => {
                        security_context = Some(SecurityContext::None);
                    },
                    "PluginSecurity" => {
                        security_context = Some(SecurityContext::PluginSecurity);
                    },
                    "LocalUserSecurity" => {
                        security_context = Some(SecurityContext::LocalUserSecurity);
                    },
                    "RobloxScriptSecurity" => {
                        security_context = Some(SecurityContext::RobloxScriptSecurity);
                    },
                    _ => {
                        return Err(pnm.error("unknown secuirty context"));
                    }
                }
            }
            _ => {
                return Err(pnm.error("bad option name"))
            }
        }
        Ok(())
    })?;
    
    Ok(
        LuaFunctionData {
            lua_name: match lua_name { Some(s) => s, None => return Err(Error::new(attr.span(), "`name` is required, but was not given")) },
            virt: virt.unwrap_or(false),
            security_context: security_context.unwrap_or(SecurityContext::None),
            asyn: asyn.unwrap_or(false),
            sig: match sig { Some(s) => s, None => return Err(Error::new(attr.span(), "`func` is required, but was not given")) },
        }
    )
}

#[derive(Debug)]
pub enum InstanceContent {
    RustField { rust_field: Field },
    LuaField { lua_field: LuaPropertyData, rust_field: Field },
}

#[derive(Debug)]
pub struct InstanceContents {
    #[allow(unused)]
    pub brace_token: Brace,
    pub named: Punctuated<InstanceContent, Comma>,
}

#[derive(Debug)]
pub struct Instance {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub struct_token: Struct,
    pub ident: Ident,
    pub generics: Generics,
    pub contents: InstanceContents,
    pub semi_token: Option<Semi>,
}

pub enum InstanceConfigAttr {
    NoClone(bool, Option<Eq>, Span),
    ParentLocked(bool, Option<Eq>, Span),
    Hierarchy(Eq, Bracket, Punctuated<syn::Path, Token![,]>, Span),
    CustomNew(bool, Option<Eq>, Span),
    RequiresInit(bool, Option<Eq>, Span)
}

macro_rules! bool_arg {
    ($input:expr => $typ:ident | $span:expr) => {
        match $input.parse::<Token![=]>() {
            Ok(tok) => {
                let b = $input.parse::<LitBool>()?;
                Ok(Self::$typ(b.value(), Some(tok), $span))
            },
            Err(_) => {
                // treat as true
                Ok(Self::$typ(true, None, $span))
            },
        }
    };
}

impl Parse for InstanceConfigAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse::<Ident>()?;
        let name = ident.to_string();

        match name.as_str() {
            "no_clone" => { return bool_arg!(input => NoClone | ident.span()) },
            "parent_locked" => { return bool_arg!(input => ParentLocked | ident.span()) },
            "custom_new" => { return bool_arg!(input => CustomNew | ident.span()) },
            "requires_init" => { return bool_arg!(input => RequiresInit | ident.span()) },
            "hierarchy" => {
                // this, will be a nightmare.
                let equals = input.parse::<Token![=]>()?;
                let content;
                let brackets = bracketed!(content in input);
                
                let punct: Punctuated<syn::Path, Token![,]> = Punctuated::parse_terminated(&content)?;

                return Ok(InstanceConfigAttr::Hierarchy(equals, brackets, punct, ident.span()))
            },
            _ => {
                return Err(Error::new(ident.span(), "unknown attribute"))
            }
        }
    }
}

#[derive(Debug)]
pub struct InstanceConfig {
    pub no_clone: bool,
    pub parent_locked: bool,
    pub hierarchy: Vec<syn::Path>,
    pub custom_new: bool,
    pub requires_init: bool
}

impl Parse for Instance {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse::<Visibility>()?;
        let struct_token = input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;
        let (where_clause, contents, semi_token) = data_struct(input)?;
        Ok(Instance {
            attrs,
            vis,
            struct_token,
            ident,
            generics: Generics {
                where_clause,
                ..generics
            },
            contents,
            semi_token,
        })
    }
}

fn search_attrs_field(mut field: Field) -> Result<InstanceContent> {
    let mut filtered: Vec<(usize, &Attribute)> = field.attrs.iter().enumerate().filter(|(_, a)| a.path().is_ident("property")).collect();

    let field_name = field.ident.clone().unwrap();

    if filtered.is_empty() {
        return Ok(InstanceContent::RustField { rust_field: field })
    }

    if filtered.len() != 1 {
        return Err(Error::new(field.span(), format!("`instance`: expected 1 `property` specifier, got {}", filtered.len())))
    } else {
        let (idx, attr) = filtered.pop().unwrap();
        let (mut name, mut readonly, mut get, mut set, mut security_context, mut default, mut not_replicated, mut transparent) = (None, None, None, None, None, None, None, None);
        if let Err(e) = attr.parse_nested_meta(|nested_meta| {
            let ident = nested_meta.path.require_ident()?;
            let ident = ident.to_string();

            match ident.as_str() {
                "name" => {
                    if name.is_none() {
                        let nname: LitStr = nested_meta.value()?.parse()?;
                        let nname = nname.value();
                        if nname.is_empty() {
                            return Err(nested_meta.error("field name cannot be empty"));
                        }
                        name = Some(nname);
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "default" => {
                    if default.is_none() {
                        if let Ok(v) = nested_meta.value() {
                            let expr: syn::Expr = v.parse()?;

                            default = Some(Some(expr));
                        } else {
                            default = Some(None);
                        }
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "readonly" => {
                    if let None = readonly {
                        if let Ok(ts) = nested_meta.value() {
                            let b: LitBool = ts.parse()?;
                            readonly = Some(b.value());
                        } else {
                            readonly = Some(true);
                        }
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "get" => {
                    if let None = get {
                        let name: Path = nested_meta.value()?.parse()?;
                        get = Some(name);
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "set" => {
                    if let None = set {
                        let name: Path = nested_meta.value()?.parse()?;
                        set = Some(name);
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "not_replicated" => {
                    if let None = not_replicated {
                        if let Ok(ts) = nested_meta.value() {
                            let b: LitBool = ts.parse()?;
                            not_replicated = Some(b.value());
                        } else {
                            not_replicated = Some(true);
                        }
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "transparent" => {
                    if let None = transparent {
                        if let Ok(ts) = nested_meta.value() {
                            let b: LitBool = ts.parse()?;
                            transparent = Some(b.value());
                        } else {
                            transparent = Some(true);
                        }
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                "security_context" => {
                    if let None = security_context {
                        let value: Ident = nested_meta.value()?.parse()?;

                        let ident = value.to_string();

                        match ident.as_str() {
                            "None" => {
                                security_context = Some(SecurityContext::None);
                            },
                            "PluginSecurity" => {
                                security_context = Some(SecurityContext::PluginSecurity);
                            },
                            "LocalUserSecurity" => {
                                security_context = Some(SecurityContext::LocalUserSecurity);
                            },
                            "RobloxScriptSecurity" => {
                                security_context = Some(SecurityContext::RobloxScriptSecurity);
                            },
                            _ => {
                                return Err(nested_meta.error("unknown secuirty context"));
                            }
                        }
                    } else {
                        return Err(nested_meta.error("already specified"));
                    }
                },
                _ => {
                    return Err(nested_meta.error("unknown attribute"));
                }
            }
            Ok(())
        }) {
            return Err(e)
        };

        let lpa = LuaPropertyData {
            name: match name {
                Some(s) => s,
                None => return Err(Error::new(attr.span(), "name must be specified")),
            },
            readonly: readonly.unwrap_or(false),
            get,
            set,
            security_context: security_context.unwrap_or(SecurityContext::None),
            default,
            not_replicated: not_replicated.unwrap_or(false),
            transparent: transparent.unwrap_or(false),
            span: field.span(),
            rust_name: field_name
        };

        field.attrs.remove(idx);

        return Ok(InstanceContent::LuaField { lua_field: lpa, rust_field: field })
    }
}

impl Parse for InstanceContent {
    fn parse(input: ParseStream) -> Result<Self> {
        let field = input.call(Field::parse_named)?;
        return Ok(search_attrs_field(field)?);
    }
}

impl Parse for InstanceContents {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(InstanceContents {
            brace_token: braced!(content in input),
            named: content.parse_terminated(InstanceContent::parse, Token![,])?,
        })
    }
}

pub(crate) fn data_struct(
    input: ParseStream,
) -> Result<(Option<WhereClause>, InstanceContents, Option<Token![;]>)> {
    let mut lookahead = input.lookahead1();
    let mut where_clause = None;
    if lookahead.peek(Token![where]) {
        where_clause = Some(input.parse()?);
        lookahead = input.lookahead1();
    }

    if lookahead.peek(token::Brace) {
        let fields = input.parse()?;
        Ok((where_clause, fields, None))
    } else {
        Err(lookahead.error())
    }
}