use std::ops::{BitAnd, BitOr, BitOrAssign};

use proc_macro2::Ident;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, ReturnType};

use super::object::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceMask {
    None = 0,
    NonConstructible = 1,   // prevents Instance.new() being bound
    NoClone = 2,            // makes cloning raise an error
    ParentChangeLocked = 4, // instantly locks the parent after setting it.
    NoDestruction = 8,      // makes Instance:Destroy() no-op
}

impl ToTokens for InstanceMask {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mask_value = *self as u8;
        tokens.extend(quote! {
            crate::core::InstanceMask::from_raw(#mask_value)
        })
    }
}

impl BitOr for InstanceMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 | rhs as u8) }
    }
}
impl BitAnd for InstanceMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 & rhs as u8) }
    }
}
impl BitOrAssign for InstanceMask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
#[derive(Debug)]
pub struct PartialInstanceClassInfo {
    pub class_name: String,
    pub hierarchy: Option<Vec<Ident>>,
    pub properties: Vec<ObjectPropertyInfo>,
    pub virtual_funcs: Vec<VirtualLuaFunctionInfo>,
    pub signals: Vec<Ident>,
    pub mask: ObjectMask,
    pub instance_mask: InstanceMask,
}

impl From<(PartialObjectClassInfo, InstanceMask)> for PartialInstanceClassInfo {
    fn from((info, instance_mask): (PartialObjectClassInfo, InstanceMask)) -> Self {
        Self {
            class_name: info.class_name,
            hierarchy: info.hierarchy,
            properties: info.properties,
            virtual_funcs: info.virtual_funcs,
            signals: info.signals,
            mask: info.mask,
            instance_mask,
        }
    }
}

#[derive(Debug)]
pub struct InstanceClassInfo {
    pub partial_info: PartialInstanceClassInfo,
    pub method_list: Vec<LuaFunctionInfo>,
}

impl From<(ObjectClassInfo, InstanceMask)> for InstanceClassInfo {
    fn from((info, instance_mask): (ObjectClassInfo, InstanceMask)) -> Self {
        Self {
            partial_info: PartialInstanceClassInfo {
                class_name: info.partial_info.class_name,
                hierarchy: info.partial_info.hierarchy,
                properties: info.partial_info.properties,
                virtual_funcs: info.partial_info.virtual_funcs,
                signals: info.partial_info.signals,
                mask: info.partial_info.mask,

                instance_mask,
            },
            method_list: info.method_list,
        }
    }
}

impl PartialInstanceClassInfo {
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
        let instance_mask = self.instance_mask;
        let signals = &self.signals;
        quote! {
            crate::core::PartialInstanceClassInfo {
                class_name: stringify!(#class_name),
                hierarchy: &[#(stringify!(#hierarchy),)*],
                properties: &[#(#properties,)*],
                virtual_funcs: &[#(#virtual_funcs,)*],
                signals: &[#(stringify!(#signals),)*],
                mask: #mask,
                instance_mask: #instance_mask,
            }
        }
    }
    pub fn export(&self) -> TokenStream {
        let info = self.export_info_only();
        let class_name = Ident::new(&self.class_name, Span::call_site());
        quote! {
            impl crate::core::PartialInstanceClass for #class_name {
                const PARTIAL_INSTANCE_CLASS_INFO: crate::core::PartialInstanceClassInfo = #info;
            }
        }
    }
}
