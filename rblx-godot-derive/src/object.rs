use core::num;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use std::fmt::Debug;
use std::mem::transmute;
use std::ops::{BitAnd, BitOr, BitOrAssign};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Not;
use syn::{Expr, ItemStruct, ReturnType, Signature, Token, Type};

use crate::parse::{AttrArg, AttrArgValue, AttrNamedArg};
use crate::utils::snake_case_to_camel_case;
use crate::AttrArguments;

#[derive(Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
pub struct SecurityContext(u8);
impl SecurityContext {
    pub const NONE: SecurityContext = SecurityContext(0);
    pub const PLUGIN: SecurityContext = SecurityContext(0x1);
    pub const ROBLOX_PLACE: SecurityContext = SecurityContext(0x2);
    pub const WRITE_PLAYER: SecurityContext = SecurityContext(0x4);
    pub const LOCAL_USER: SecurityContext = SecurityContext(0x10);
    pub const ROBLOX_SCRIPT: SecurityContext = SecurityContext(0x1D);
    pub const ROBLOX: SecurityContext = SecurityContext(0x1F);
    pub const TEST_LOCAL_USER: SecurityContext = SecurityContext(0x10);
}

impl BitOr for SecurityContext {
    type Output = SecurityContext;
    fn bitor(self, rhs: Self) -> Self::Output {
        SecurityContext(self.0 | rhs.0)
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        SecurityContext::NONE
    }
}

impl SecurityContext {
    pub const fn has(self, other: SecurityContext) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Into<u8> for SecurityContext {
    fn into(self) -> u8 {
        self.0
    }
}
impl Debug for SecurityContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NONE => write!(f, "SecurityContext(NONE)"),
            Self::PLUGIN => write!(f, "SecurityContext(PLUGIN)"),
            Self::LOCAL_USER => write!(f, "SecurityContext(LOCAL_USER)"),
            Self::ROBLOX_SCRIPT => write!(f, "SecurityContext(ROBLOX_SCRIPT)"),
            Self::ROBLOX => write!(f, "SecurityContext(ROBLOX)"),
            _ => write!(f, "SecurityContext(0x{:x})", self.0),
        }
    }
}
impl ToTokens for SecurityContext {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Self::NONE => tokens.extend(quote! { crate::core::SecurityContext::NONE }),
            Self::PLUGIN => tokens.extend(quote! { crate::core::SecurityContext::PLUGIN }),
            Self::LOCAL_USER => tokens.extend(quote! { crate::core::SecurityContext::LOCAL_USER }),
            Self::ROBLOX_SCRIPT => {
                tokens.extend(quote! { crate::core::SecurityContext::ROBLOX_SCRIPT })
            }
            Self::ROBLOX => tokens.extend(quote! { crate::core::SecurityContext::ROBLOX }),
            _ => tokens.extend(quote! { #self }),
        }
    }
}

impl From<u8> for SecurityContext {
    fn from(val: u8) -> Self {
        SecurityContext(val)
    }
}

#[derive(Debug, Clone)]
pub struct LuaPropertyInfo {
    pub name: String,
    pub readonly: bool,
    pub security_context: SecurityContext,
    pub not_replicated: bool,
    pub get: Option<Ident>,
    pub set: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LuaFunctionInfo {
    pub lua_name: String,
    pub security_context: SecurityContext,
    pub sig: Signature,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VirtualLuaFunctionInfo {
    pub func_info: LuaFunctionInfo,
    pub is_overriding: Option<Ident>,
    pub base_class: Ident,
}
#[derive(Debug, Clone)]
pub enum ObjectPropertyInfo {
    Rust {
        name: Ident,
        ty: Type,
        default: Option<Expr>,
    },
    Transparent(LuaPropertyInfo),
    Lua {
        name: Ident,
        ty: Type,
        default: Option<Expr>,
        info: LuaPropertyInfo,
    },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// Implementation details about this Object class.
pub enum ObjectMask {
    None = 0,

    /// If enabled, calls `_init` to construct the object when `new` is called.
    /// or else uses the default constructor.
    RequiresInit = 1,

    /// Whether signals are handled by an instance constructor or by the class itself.
    Signals = 2,

    /// Whether there is a function named _component_init to construct the component.
    ComponentInit = 4,

    /// Non-constructible object.
    Abstract = 8,
}

impl BitOr for ObjectMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 | rhs as u8) }
    }
}
impl BitAnd for ObjectMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 & rhs as u8) }
    }
}
impl BitOrAssign for ObjectMask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
#[derive(Debug, Clone)]
pub struct PartialObjectClassInfo {
    pub class_name: String,
    pub hierarchy: Option<Vec<Ident>>,
    pub properties: Vec<ObjectPropertyInfo>,
    pub virtual_funcs: Vec<VirtualLuaFunctionInfo>,
    pub signals: Vec<Ident>,
    pub mask: ObjectMask,
}

impl ToTokens for ObjectMask {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let num = *self as u8;
        tokens.extend(quote! {
            crate::core::ObjectMask::create_raw(#num)
        });
    }
}

impl PartialObjectClassInfo {
    pub fn new(class_name: &Ident, hierarchy: Option<&Vec<Ident>>) -> Self {
        Self {
            class_name: class_name.to_string(),
            hierarchy: hierarchy.cloned(),
            properties: Vec::new(),
            virtual_funcs: Vec::new(),
            signals: Vec::new(),
            mask: ObjectMask::None,
        }
    }
    pub fn populate_table(
        &mut self,
        mask: ObjectMask,
        item: &ItemStruct,
    ) -> Result<(), syn::Error> {
        self.mask = mask;
        self.virtual_funcs = item.attrs
            .iter()
            .map(|attr| {
                match &attr.meta {
                    syn::Meta::List(m) => {
                        let name = m.path.get_ident().expect("Expected attribute name");
                        match name.to_string().as_str() {
                            "method" => {
                                let args: AttrArguments = syn::parse2(m.tokens.clone()).expect("Failed to parse attribute arguments");
                                let is_overriding: Option<Ident> = match args.get_arg(0).expect("Expected `virtual` or `override` argument") {
                                    AttrArg::Unnamed(AttrArgValue::Expr(syn::Expr::Lit(syn::ExprLit {lit: syn::Lit::Str(s),..}))) => match s.value().as_str() {
                                        "override" => Some(Ident::new(&self.class_name, Span::call_site())),
                                        "virtual" => None,
                                        _ => panic!("Expected `virtual` or `override` argument"),
                                    },
                                    AttrArg::Named(AttrNamedArg { name, value, ..}) => match name.to_string().as_str() {
                                        "override" => Some(match value {
                                            AttrArgValue::Expr(syn::Expr::Path(p)) =>
                                                p.path.get_ident().expect("Expected `override=<ident>` argument").clone(),
                                            _ => panic!("Expected `override=<ident>` argument"),
                                        }),
                                        _ => panic!("Expected `override=` argument"),
                                    },
                                    _ => panic!("Expected `virtual` or `override` argument")
                                };
                                let sig = args.get_unnamed_arg(1).expect("Expected function signature");
                                let sig = match sig {
                                    AttrArgValue::Func(s) => s,
                                    _ => panic!("Expected function signature")
                                };
                                let security_context = args.get_named_arg("security_context").map(|v| {
                                    match &v.value {
                                        AttrArgValue::Expr(syn::Expr::Path(p)) =>
                                            p.path.get_ident().map_or(SecurityContext::NONE, |i| match i.to_string().as_str() {
                                                "None" => SecurityContext::NONE,
                                                "Plugin" => SecurityContext::PLUGIN,
                                                "RobloxPlace" => SecurityContext::ROBLOX_PLACE,
                                                "WritePlayer" => SecurityContext::WRITE_PLAYER,
                                                "LocalUser" => SecurityContext::LOCAL_USER,
                                                "RobloxScript" => SecurityContext::ROBLOX_SCRIPT,
                                                "Roblox" => SecurityContext::ROBLOX,
                                                _ => panic!("Invalid security context: {:?}", i),
                                            }),
                                        _ => panic!("Expected security context")
                                    }
                                }).unwrap_or(SecurityContext::NONE);
                                let base_class;
                                if is_overriding.is_some() {
                                    base_class = args.get_named_arg("base").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(syn::Expr::Path(p)) => p.path.get_ident().unwrap().clone(),
                                            _ => panic!("Expected base class")
                                        }
                                    }).expect("Expected base class");
                                } else {
                                    base_class = Ident::new(self.class_name.as_str(), Span::call_site());
                                }
                                VirtualLuaFunctionInfo {
                                    func_info: LuaFunctionInfo {
                                        lua_name: snake_case_to_camel_case(sig.ident.to_string().as_str()),
                                        security_context,
                                        sig: sig.clone(),
                                        span: attr.span()
                                    },
                                    is_overriding,
                                    base_class
                                }
                            },
                            _ => {
                                panic!("Invalid attribute for object class (expected virtual func def): {:?}", attr);
                            }
                        }
                    },
                    _ => {
                        panic!("Invalid attribute for object class (expected virtual func def): {:?}", attr);
                    }
                }
            })
            .collect();
        self.signals.clear();
        self.properties = item
            .fields
            .iter()
            .map(|field| {
                field
                    .attrs
                    .iter()
                    .map(|attr| match &attr.meta {
                        syn::Meta::List(m) => m
                            .path
                            .get_ident()
                            .map(|name| match name.to_string().as_str() {
                                "lua_property" => {
                                    let args: AttrArguments = syn::parse2(m.tokens.clone()).expect("Failed to parse attribute arguments");
                                    let is_transparent = args.has_unnamed_arg("transparent");
                                    let readonly = args.has_unnamed_arg("readonly");
                                    let not_replicated = args.has_unnamed_arg("not_replicated");

                                    let name = args.get_named_arg("name").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(syn::Expr::Lit(lit)) => {
                                                match &lit.lit {
                                                    syn::Lit::Str(s) => s.value(),
                                                    _ => panic!("Expected string literal for property name"),
                                                }
                                            },
                                            _ => panic!("Expected string literal for property name"),
                                        }
                                    }).unwrap_or_else(|| snake_case_to_camel_case(field.ident.as_ref().unwrap().to_string().as_str()));
                                    let security_context = args.get_named_arg("security_context").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(syn::Expr::Path(p)) =>
                                                p.path.get_ident().map_or(SecurityContext::NONE, |i| match i.to_string().as_str() {
                                                    "None" => SecurityContext::NONE,
                                                    "Plugin" => SecurityContext::PLUGIN,
                                                    "RobloxPlace" => SecurityContext::ROBLOX_PLACE,
                                                    "WritePlayer" => SecurityContext::WRITE_PLAYER,
                                                    "LocalUser" => SecurityContext::LOCAL_USER,
                                                    "RobloxScript" => SecurityContext::ROBLOX_SCRIPT,
                                                    "Roblox" => SecurityContext::ROBLOX,
                                                    _ => panic!("Invalid security context: {:?}", i),
                                                }),
                                            _ => panic!("Expected security context")
                                        }
                                    }).unwrap_or(SecurityContext::NONE);
                                    let get = args.get_named_arg("get").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(syn::Expr::Path(p)) => p.path.get_ident().unwrap().clone(),
                                            _ => panic!("Expected getter function name"),
                                        }
                                    });
                                    let set = args.get_named_arg("set").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(syn::Expr::Path(p)) => p.path.get_ident().unwrap().clone(),
                                            _ => panic!("Expected setter function name"),
                                        }
                                    });
                                    let default = args.get_named_arg("default").map(|v| {
                                        match &v.value {
                                            AttrArgValue::Expr(e) => e.clone(),
                                            _ => panic!("Expected default value"),
                                        }
                                    });
                                    if readonly && set.is_some() {
                                        panic!("Readonly properties cannot have a setter function");
                                    }
                                    if is_transparent {
                                        if get.is_none() {
                                            panic!("Transparent properties must have a getter function");
                                        }
                                        if !readonly && set.is_none() {
                                            panic!("Transparent properties must have a setter function if not readonly");
                                        }
                                        Some(ObjectPropertyInfo::Transparent(LuaPropertyInfo {
                                            name,
                                            readonly,
                                            security_context,
                                            not_replicated,
                                            get: get.clone(),
                                            set: set.clone(),
                                            span: attr.span(),
                                        }))
                                    } else {
                                        Some(ObjectPropertyInfo::Lua {
                                            name: field.ident.as_ref().unwrap().clone(),
                                            ty: field.ty.clone(),
                                            default: default.clone(),
                                            info: LuaPropertyInfo {
                                                name,
                                                readonly,
                                                security_context,
                                                not_replicated,
                                                get,
                                                set,
                                                span: attr.span(),
                                            }
                                        })
                                    }
                                },
                                _ => None,
                            })
                            .unwrap_or(None),
                        _ => None,
                    })
                    .fold(None, |acc, a| match (acc, a) {
                        (Some(v), None) | (None, Some(v)) => Some(v),
                        (Some(_), Some(_)) => panic!(
                            "Property {} was registered twice.",
                            field.ident.as_ref().unwrap()
                        ),
                        _ => None,
                    })
                    .unwrap_or_else(|| ObjectPropertyInfo::Rust {
                        name: field.ident.as_ref().unwrap().clone(),
                        ty: field.ty.clone(),
                        default: None,
                    })
            })
            .filter_map(|x| {
                match x {
                    ObjectPropertyInfo::Lua { name, ty, info, default } => match ty {
                        Type::Path(p) if p.path.is_ident("RBXScriptSignal") => {
                            self.signals.push(name.clone());
                            return None
                        },
                        _ => Some(ObjectPropertyInfo::Lua { name, ty, default, info })
                    },
                    x => Some(x)
                }
            })
            .collect();
        Ok(())
    }
    pub fn export(&self) -> TokenStream {
        let class_name = Ident::new(&self.class_name, Span::call_site());
        let info = self.export_info_only();
        quote! {
            impl crate::core::PartialObjectClass for #class_name {
                const PARTIAL_OBJECT_CLASS_INFO: crate::core::PartialObjectClassInfo = #info;
            }
        }
    }
    pub fn export_info_only(&self) -> TokenStream {
        let class_name = Ident::new(&self.class_name, Span::call_site());
        let hierarchy = self.hierarchy.clone().unwrap_or_default();
        let properties: Vec<TokenStream> = self
            .properties
            .iter()
            .map(|x| match x {
                ObjectPropertyInfo::Lua {
                    name,
                    ty,
                    default,
                    info,
                } => {
                    let default = default
                        .as_ref()
                        .map(|d| {
                            quote! {
                                Some(#d)
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    let name = &info.name;
                    let readonly = info.readonly;
                    let security_context = info.security_context;
                    let not_replicated = info.not_replicated;
                    let get = info
                        .get
                        .as_ref()
                        .map(|g| {
                            quote! {
                                Some(stringify!(#g))
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    let set = info
                        .set
                        .as_ref()
                        .map(|s| {
                            quote! {
                                Some(stringify!(#s))
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    quote! {
                        crate::core::ObjectPropertyInfo::Lua {
                            name: stringify!(#name),
                            ty: stringify!(#ty),
                            default: #default,
                            info: crate::core::LuaPropertyInfo {
                                name: #name,
                                readonly: #readonly,
                                security_context: #security_context,
                                not_replicated: #not_replicated,
                                get: Some(stringify!(#get)),
                                set: Some(stringify!(#set)),
                            }
                        }
                    }
                }
                ObjectPropertyInfo::Rust { name, ty, default } => {
                    let default = default
                        .as_ref()
                        .map(|d| {
                            quote! {
                                Some(#d)
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    quote! {
                        crate::core::ObjectPropertyInfo::Rust {
                            name: stringify!(#name),
                            ty: stringify!(#ty),
                            default: #default,
                        }
                    }
                }
                ObjectPropertyInfo::Transparent(info) => {
                    let name = &info.name;
                    let readonly = info.readonly;
                    let security_context = info.security_context;
                    let not_replicated = info.not_replicated;
                    let get = info
                        .get
                        .as_ref()
                        .map(|g| {
                            quote! {
                                Some(stringify!(#g))
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    let set = info
                        .set
                        .as_ref()
                        .map(|s| {
                            quote! {
                                Some(stringify!(#s))
                            }
                        })
                        .unwrap_or(quote! {
                            None
                        });
                    quote! {
                        crate::core::ObjectPropertyInfo::Transparent(
                            crate::core::LuaPropertyInfo {
                                name: stringify!(#name),
                                readonly: #readonly,
                                security_context: #security_context,
                                not_replicated: #not_replicated,
                                get: #get,
                                set: #set,
                            }
                        )
                    }
                }
            })
            .collect();
        let virtual_funcs: Vec<TokenStream> = self
            .virtual_funcs
            .iter()
            .map(|x| {
                let base_class = &x.base_class;
                let is_overriding = x.is_overriding.clone();
                let lua_name = Ident::new(x.func_info.lua_name.as_str(), Span::call_site());
                let security_context = x.func_info.security_context;
                let sig_name = &x.func_info.sig.ident;
                let sig_ret_type = if let ReturnType::Type(_, ty) = &x.func_info.sig.output {
                    Ident::new(ty.to_token_stream().to_string().as_str(), Span::call_site())
                } else {
                    Ident::new("()", Span::call_site())
                };
                let sig_args = x
                    .func_info
                    .sig
                    .inputs
                    .iter()
                    .map(|arg| Ident::new(arg.to_token_stream().to_string().as_str(), arg.span()))
                    .collect::<Vec<_>>();
                quote! {
                    crate::core::VirtualLuaFunctionInfo {
                        func_info: crate::core::LuaFunctionInfo {
                            lua_name: stringify!(#lua_name),
                            security_context: #security_context,
                            sig: crate::core::LuaFunctionSignature {
                                name: stringify!(#sig_name),
                                args: &[#(stringify!(#sig_args)),*],
                                ret: stringify!(#sig_ret_type),
                            },
                        },
                        is_overriding: #is_overriding,
                        base_class: stringify!(#base_class)
                    }
                }
            })
            .collect();
        let mask = self.mask;
        let signals = &self.signals;
        quote! {
            crate::core::PartialObjectClassInfo {
                class_name: stringify!(#class_name),
                hierarchy: &[#(stringify!(#hierarchy),)*],
                properties: &[#(#properties,)*],
                virtual_funcs: &[#(#virtual_funcs,)*],
                signals: &[#(stringify!(#signals),)*],
                mask: #mask,
            }
        }
    }
}

impl ToTokens for PartialObjectClassInfo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.export());
    }
}

#[derive(Debug, Clone)]
pub struct ObjectClassInfo {
    pub partial_info: PartialObjectClassInfo,
    pub method_list: Vec<LuaFunctionInfo>,
}
