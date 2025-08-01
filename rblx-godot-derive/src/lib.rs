#![feature(if_let_guard)]

use std::collections::HashMap;

use convert_case::Casing;
use inheritance::create_inheritance_map;
use object::{ObjectMask, PartialObjectClassInfo, VirtualLuaFunctionInfo};
use parse::AttrArguments;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, Error, ExprArray,
    Field, Ident, ImplItem, ItemEnum, ItemImpl, ItemStruct, LitStr, Path, PathSegment, Token,
    TraitBound, TypeParamBound, TypeTraitObject,
};
use utils::camel_case_to_snake_case;

use crate::{
    instance::{InstanceMask, PartialInstanceClassInfo},
    object::{LuaFunctionInfo, ObjectPropertyInfo, SecurityContext},
    parse::{AttrArgValue, AttrNamedArg},
    utils::snake_case_to_camel_case,
};

mod inheritance;
mod instance;
mod object;
mod parse;
mod utils;

macro_rules! error_cattr {
    ($span:expr, $err:literal) => {
        syn::Error::new($span, $err).into_compile_error()
    };
}
/*
#[proc_macro_attribute]
pub fn object_methods(
    _item: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut impl_block: ItemImpl = parse_macro_input!(ts);

    if let syn::Type::Path(ref p) = *impl_block.self_ty {
        let Some(id) = p.path.get_ident() else {
            return Error::new(impl_block.self_ty.span(), "name not ident")
                .into_compile_error()
                .into();
        };
        let ident = "I".to_owned() + &id.to_string();

        let mut path = Punctuated::new();
        path.push(PathSegment {
            ident: Ident::new(&ident, id.span()),
            arguments: syn::PathArguments::None,
        });

        let mut p = Punctuated::new();
        p.push(TypeParamBound::Trait(TraitBound {
            paren_token: None,
            modifier: syn::TraitBoundModifier::None,
            lifetimes: None,
            path: Path {
                leading_colon: None,
                segments: path,
            },
        }));

        impl_block.self_ty = Box::new(syn::Type::TraitObject(TypeTraitObject {
            dyn_token: Some(Token![dyn](id.span())),
            bounds: p,
        }));

        return impl_block.to_token_stream().into();
    } else {
        return Error::new(impl_block.self_ty.span(), "unknown type name kind")
            .into_compile_error()
            .into();
    }
}
*/
#[proc_macro_attribute]
pub fn lua_enum(
    arguments: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let enum_block: ItemEnum = parse_macro_input!(ts);
    let name = enum_block.ident.clone();
    let vis = enum_block.vis.clone();

    let args: AttrArguments = parse_macro_input!(arguments);

    let default_impl = if let Some(default) = args.get_named_arg("default") {
        let default_val = &default.value;
        let default_name: Ident = {
            let default_name_arg = args.get_named_arg("default_name").map(|x| x.value.clone());
            match default_name_arg {
                Some(x) => match x.try_into() {
                    Ok(x) => x,
                    Err(e) => {
                        return e.into_compile_error().into();
                    }
                },
                None => Ident::new("Default", Span::call_site()),
            }
        };
        quote! {
            impl Default for #name {
                fn default() -> Self {
                    Self::#default_name
                }
            }

            impl #name {
                pub const fn collapse_default(self) -> Self {
                    match self {
                        Self::#default_name => Self::#default_val,
                        x => x
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let mut last_index: Option<usize> = None;
    let mut variants = vec![];

    for x in enum_block.variants.iter() {
        match x.discriminant.as_ref().map(|(_, expr)| expr) {
            Some(e)
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(i),
                    ..
                }) = e =>
            {
                last_index = Some(i.base10_parse().unwrap());
                variants.push((x.ident.to_string(), last_index.unwrap() as i16));
            }
            None => match last_index {
                Some(i) => {
                    last_index = Some(i + 1);
                    variants.push((x.ident.to_string(), last_index.unwrap() as i16));
                }
                None => {
                    last_index = Some(0);
                    variants.push((x.ident.to_string(), 0));
                }
            },
            _ => {
                return syn::Error::new(x.span(), "invalid discriminant")
                    .into_compile_error()
                    .into();
            }
        }
    }

    let variant_quotes_names: Vec<TokenStream> = variants
        .iter()
        .map(|(variant_name, _value)| {
            let variant_name = Ident::new(variant_name, Span::call_site());
            quote! {
                Self::#variant_name => concat!(stringify!(#name), ".", stringify!(#variant_name))
            }
        })
        .collect();
    let variant_quotes_names_only: Vec<TokenStream> = variants
        .iter()
        .map(|(variant_name, _value)| {
            let variant_name = Ident::new(variant_name, Span::call_site());
            quote! {
                Self::#variant_name => stringify!(#variant_name)
            }
        })
        .collect();

    let variant_fields: Vec<TokenStream> = variants
        .iter()
        .map(|(variant_name, _value)| {
            let variant_name = Ident::new(variant_name, Span::call_site());
            quote! {
                fields.add_field(stringify!(#variant_name), #name::#variant_name);
            }
        })
        .collect();

    let enum_type_name = Ident::new(&format!("LuaEnum{}", name.to_string()), Span::call_site());

    quote! {
        use r2g_mlua::prelude::*;

        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
        #[repr(i16)]
        #enum_block

        impl FromLua for #name {
            fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
                let ud = value.as_userdata();
                if ud.is_none() {
                    Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: "EnumItem".into(),
                        message: None,
                    })
                } else {
                    let unwrapped = unsafe { ud.unwrap_unchecked() }.borrow::<Self>();
                    if unwrapped.is_err() {
                        Err(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: "EnumItem".into(),
                            message: None,
                        })
                    } else {
                        unsafe { Ok(*unwrapped.unwrap_unchecked()) }
                    }
                }
            }
        }

        impl LuaUserData for #name {
            fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
                methods.add_meta_method("__tostring", |_, this, ()| {
                    Ok(String::from(match *this {
                        #(#variant_quotes_names),*
                    }))
                });
            }
            fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
                fields.add_meta_field("__type", "EnumItem");
                fields.add_field_method_get("Name", |_, this| Ok(match (this) {
                    #(#variant_quotes_names_only),*
                }));
                fields.add_field_method_get("Value", |_, this| Ok(*this as i16));

            }
        }

        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
        #vis struct #enum_type_name;

        impl FromLua for #enum_type_name {
            fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
                let ud = value.as_userdata();
                if ud.is_none() {
                    Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: "Enum".into(),
                        message: None,
                    })
                } else {
                    let unwrapped =
                        unsafe { ud.unwrap_unchecked() }.borrow::<Self>();
                    if unwrapped.is_err() {
                        Err(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: "Enum".into(),
                            message: None,
                        })
                    } else {
                        unsafe { Ok(*unwrapped.unwrap_unchecked()) }
                    }
                }
            }
        }

        impl LuaUserData for #enum_type_name {
            fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
                methods.add_meta_method("__tostring", |_, _, ()| {
                    Ok(String::from(concat!("Enums.",stringify!(#name))))
                });
            }
            fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
                fields.add_meta_field("__type", "Enum");
                #(#variant_fields)*
            }
        }

        #default_impl
    }
    .into()
}

#[doc(hidden)]
#[proc_macro]
pub fn create_enums(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let wrapper: ExprArray = parse_macro_input!(ts);
    let mut enums: Vec<Ident> = vec![];
    for token in wrapper.elems.iter() {
        if let syn::Expr::Path(p) = token {
            if let Some(i) = p.path.get_ident() {
                enums.push(i.clone());
            } else {
                return syn::Error::new(p.path.span(), "not an ident")
                    .into_compile_error()
                    .into();
            }
        } else {
            return syn::Error::new(token.span(), "not a path")
                .into_compile_error()
                .into();
        }
    }

    let modules: Vec<TokenStream> = enums
        .iter()
        .map(|x| {
            let ident = Ident::new(&camel_case_to_snake_case(x.to_string().as_str()), x.span());
            quote! {
                mod #ident;
            }
        })
        .collect();

    let enum_use: Vec<TokenStream> = enums
        .iter()
        .map(|x| {
            let ident = Ident::new(&camel_case_to_snake_case(x.to_string().as_str()), x.span());
            let enum_type_name =
                Ident::new(&format!("LuaEnum{}", x.to_string()), Span::call_site());
            quote! {
                pub use #ident::#x;
                pub use #ident::#enum_type_name;
            }
        })
        .collect();

    let enum_fields: Vec<TokenStream> = enums
        .iter()
        .map(|x| {
            let enum_type_name =
                Ident::new(&format!("LuaEnum{}", x.to_string()), Span::call_site());
            quote! {
                fields.add_field(stringify!(#x), #enum_type_name);
            }
        })
        .collect();

    quote! {
        use r2g_mlua::prelude::*;

        #(#modules)*
        #(#enum_use)*

        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
        pub struct LuaEnums;

        impl FromLua for LuaEnums {
            fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
                let ud = value.as_userdata();
                if ud.is_none() {
                    Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: "Enums".into(),
                        message: None,
                    })
                } else {
                    let unwrapped = unsafe { ud.unwrap_unchecked() }.borrow::<LuaEnums>();
                    if unwrapped.is_err() {
                        Err(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: "Enums".into(),
                            message: None,
                        })
                    } else {
                        unsafe { Ok(*unwrapped.unwrap_unchecked()) }
                    }
                }
            }
        }

        impl LuaUserData for LuaEnums {
            fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
                methods.add_meta_method("__tostring", |_, _, ()| {
                    Ok(String::from("Enums"))
                });
            }
            fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
                fields.add_meta_field("__type", "Enums");
                #(#enum_fields)*
            }
        }

        impl crate::userdata::LuaSingleton for LuaEnums {
            fn register_singleton(lua: &Lua) -> LuaResult<()> {
                lua.globals().raw_set("Enums", LuaEnums)?;
                Ok(())
            }
        }

    }
    .into()
}

#[proc_macro_attribute]
pub fn object_class(
    arguments: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args: AttrArguments = parse_macro_input!(arguments);
    let mut struct_block: ItemStruct = parse_macro_input!(ts);
    let vis = struct_block.vis.clone();

    let mut hierarchy;
    if let Some(h) = args.get_named_arg("hierarchy") {
        if let Ok(expr) = h.value.clone().try_into() {
            if let syn::Expr::Array(arr) = expr {
                let mut v = Vec::new();
                for e in arr.elems.iter() {
                    if let syn::Expr::Path(p) = e {
                        if let Some(i) = p.path.get_ident() {
                            v.push(i.clone());
                        } else {
                            return syn::Error::new(p.path.span(), "not an ident")
                                .into_compile_error()
                                .into();
                        }
                    } else {
                        return syn::Error::new(e.span(), "not a path")
                            .into_compile_error()
                            .into();
                    }
                }
                hierarchy = v;
            } else {
                return syn::Error::new(h.value.span(), "not an array")
                    .into_compile_error()
                    .into();
            }
        } else {
            return syn::Error::new(h.value.span(), "not an expression")
                .into_compile_error()
                .into();
        }
    } else {
        hierarchy = vec![];
    }
    hierarchy.push(Ident::new("Object", Span::call_site()));
    let hierarchy = hierarchy;

    let mut first_flags = ObjectMask::None;

    {
        // Verify all flags
        if args.has_unnamed_arg("requires_init") {
            first_flags |= ObjectMask::RequiresInit;
        }
        if args.has_unnamed_arg("abstract") {
            first_flags |= ObjectMask::Abstract;
        }
        if args.has_unnamed_arg("signal_behavior_override") {
            first_flags |= ObjectMask::Signals;
        }
        if args.has_unnamed_arg("component_requires_init") {
            first_flags |= ObjectMask::ComponentInit;
        }
    }

    let class_name = struct_block.ident.clone();
    let component_class_name = Ident::new(
        &format!("{}Component", class_name.to_string()),
        class_name.span(),
    );
    let trait_name = Ident::new(&format!("I{}", class_name.to_string()), class_name.span());

    let mut object_info: PartialObjectClassInfo =
        PartialObjectClassInfo::new(&class_name, Some(&hierarchy));
    object_info.populate_table(first_flags, &struct_block);

    struct_block.ident = component_class_name.clone();

    let virtual_funcs = object_info
        .virtual_funcs
        .iter()
        .map(|x| (x.base_class.clone(), x.clone()))
        .fold(
            HashMap::new(),
            |mut acc: HashMap<Ident, Vec<VirtualLuaFunctionInfo>>, x| {
                acc.entry(x.0).or_insert_with(Vec::new).push(x.1);
                acc
            },
        );

    let inheritance_base_block = create_inheritance_map(&class_name, &hierarchy);
    let hierarchy_inherit_block = hierarchy.iter().map(|x| {
        if x.to_string() == "Object" || x.to_string() == "Instance" {
            return quote! {};
        }
        let trait_name = Ident::new(&format!("I{}", x.to_string()), x.span());
        let snake_case = camel_case_to_snake_case(x.to_string().as_str());
        let fn_name = Ident::new(&format!("get_{}_component", snake_case), x.span());
        let mut_fn_name = Ident::new(&format!("get_{}_component_mut", snake_case), x.span());
        let component_name = Ident::new(&format!("{}_component", snake_case), x.span());
        let component_type = Ident::new(&format!("{}Component", x.to_string()), x.span());
        let functions = virtual_funcs.get(x)
            .map(|x| {
                x.iter()
                    .filter(|f| f.is_overriding.is_some())
                    .map(|f| {
                        let sig = f.func_info.sig.clone();
                        let impl_name = Ident::new(&format!("_{}_impl", f.func_info.sig.ident), f.func_info.sig.ident.span());
                        let span = f.func_info.span;
                        let override_class_name = f.is_overriding.clone().unwrap();
                        let arg_names = f.func_info.sig.inputs.iter()
                            .filter_map(|x| {
                                if let syn::FnArg::Typed(p) = x {
                                    Some(p.pat.clone())
                                } else {
                                    None
                                }
                            })
                            .map(|x| x.to_token_stream())
                            .collect::<Vec<_>>();
                        quote_spanned! {span=>
                            #sig {
                                #override_class_name::#impl_name(self, #(#arg_names),*)
                            }
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        quote! {
            impl #trait_name for #class_name {
                fn #fn_name(&self) -> crate::core::RwLockReadGuard<'_, crate::instance::#component_type> {
                    self.#component_name.read().unwrap()
                }
                fn #mut_fn_name(&self) -> crate::core::RwLockWriteGuard<'_, crate::instance::#component_type> {
                    self.#component_name.write().unwrap()
                }
                #(#functions)*
            }
        }
    });
    let struct_block_with_components;
    {
        let hierarchy_components: Vec<TokenStream> = hierarchy
            .iter()
            .filter(|x| x.to_string() != "Object")
            .map(|x| {
                let snake_case = camel_case_to_snake_case(x.to_string().as_str());
                let component_name = Ident::new(&format!("{}_component", snake_case), x.span());
                let component_type = Ident::new(&format!("{}Component", x.to_string()), x.span());
                quote! {
                    #component_name: crate::core::RwLock<crate::instance::#component_type>,
                }
            })
            .collect();
        let self_component_name = Ident::new(
            &format!(
                "{}_component",
                camel_case_to_snake_case(class_name.to_string().as_str())
            ),
            class_name.span(),
        );
        struct_block_with_components = quote! {
            struct #class_name {
                #(#hierarchy_components)*
                #self_component_name: crate::core::RwLock<#component_class_name>,
            }
        };
    }
    let trait_block;
    {
        let fn_name = Ident::new(
            &format!(
                "get_{}_component",
                camel_case_to_snake_case(class_name.to_string().as_str())
            ),
            class_name.span(),
        );
        let mut_fn_name = Ident::new(
            &format!(
                "get_{}_component_mut",
                camel_case_to_snake_case(class_name.to_string().as_str())
            ),
            class_name.span(),
        );
        let self_component_name = Ident::new(
            &format!(
                "{}_component",
                camel_case_to_snake_case(class_name.to_string().as_str())
            ),
            class_name.span(),
        );
        trait_block = quote! {
            trait #trait_name {
                fn #fn_name(&self) -> crate::core::RwLockReadGuard<'_, #component_class_name>;
                fn #mut_fn_name(&self) -> crate::core::RwLockWriteGuard<'_, #component_class_name>;
            }
            impl #trait_name for #class_name {
                fn #fn_name(&self) -> crate::core::RwLockReadGuard<'_, #component_class_name> {
                    self.#self_component_name.read().unwrap()
                }
                fn #mut_fn_name(&self) -> crate::core::RwLockWriteGuard<'_, #component_class_name> {
                    self.#self_component_name.write().unwrap()
                }
            }
        }
    }
    let iobject_impl_block;
    {
        let signals_defs_block;
        let lua_get_set_block;
        if first_flags & ObjectMask::Signals != ObjectMask::None {
            signals_defs_block = quote! {
                fn get_changed_signal(&self) -> crate::userdata::ManagedRBXScriptSignal {
                    self.get_changed_signal_impl()
                }
                fn get_property_changed_signal(&self, property: String) -> crate::userdata::ManagedRBXScriptSignal {
                    self.get_property_changed_signal_impl(property)
                }
            }
        } else {
            signals_defs_block = quote! {
                fn get_changed_signal(&self) -> crate::userdata::ManagedRBXScriptSignal {
                    self.get_instance_component().changed.clone()
                }
                fn get_property_changed_signal(&self, property: String) -> crate::userdata::ManagedRBXScriptSignal {
                    self.get_instance_component().get_property_changed_signal(property).unwrap()
                }
            }
        }
        if hierarchy
            .get(hierarchy.len() - 2)
            .map(|x| x.to_string() == "Instance")
            .unwrap_or(false)
        {
            // There is an instance component at the end. (This is an instance-deriving class)
            let hierarchy_based_component_list_get: Vec<TokenStream> = hierarchy
                [..hierarchy.len() - 2]
                .iter()
                .map(|x| {
                    let component_name = Ident::new(
                        &format!(
                            "{}_component",
                            camel_case_to_snake_case(x.to_string().as_str())
                        ),
                        x.span(),
                    );
                    quote! {
                        .or_else(|| self.#component_name.read().unwrap().lua_get(self, lua, &name))
                    }
                })
                .collect();
            let hierarchy_based_component_list_set: Vec<TokenStream> = hierarchy[..hierarchy.len() - 2]
                .iter()
                .map(|x| {
                    let component_name = Ident::new(
                        &format!(
                            "{}_component",
                            camel_case_to_snake_case(x.to_string().as_str())
                        ),
                        x.span(),
                    );
                    quote! {
                        .or_else(|| self.#component_name.read().unwrap().lua_set(self, lua, &name, &val))
                    }
                })
                .collect();
            let self_component_name = Ident::new(
                &format!(
                    "{}_component",
                    camel_case_to_snake_case(class_name.to_string().as_str())
                ),
                class_name.span(),
            );
            lua_get_set_block = quote! {
                fn lua_get(&self, lua: &r2g_mlua::Lua, name: String) -> r2g_mlua::prelude::LuaResult<r2g_mlua::prelude::LuaValue> {
                    use crate::core::IObjectComponent;
                    self.#self_component_name.read().unwrap().lua_get(self, lua, &name)
                        #(#hierarchy_based_component_list_get)*
                        .unwrap_or_else(|| self.instance_component.read().unwrap().lua_get(lua, &name))
                }
                fn lua_set(&self, lua: &r2g_mlua::Lua, name: String, val: r2g_mlua::prelude::LuaValue) -> r2g_mlua::prelude::LuaResult<()> {
                    use crate::core::IObjectComponent;
                    self.#self_component_name.write().unwrap().lua_set(self, lua, &name, &val)
                        #(#hierarchy_based_component_list_set)*
                        .unwrap_or_else(|| self.instance_component.write().unwrap().lua_set(lua, &name, val))
                }
            };
        } else {
            // There is no instance component at the end. (This is a non-instance-deriving class)
            let hierarchy_based_component_list: Vec<TokenStream> = hierarchy[..hierarchy.len() - 1]
                .iter()
                .map(|x| {
                    let component_name = Ident::new(
                        &format!(
                            "{}_component",
                            camel_case_to_snake_case(x.to_string().as_str())
                        ),
                        x.span(),
                    );
                    quote! {
                        .or_else(|| self.#component_name.read().unwrap().lua_get(self, lua, &name))
                    }
                })
                .collect();
            let self_component_name = Ident::new(
                &format!(
                    "{}_component",
                    camel_case_to_snake_case(class_name.to_string().as_str())
                ),
                class_name.span(),
            );
            lua_get_set_block = quote! {
                fn lua_get(&self, lua: &r2g_mlua::Lua, name: String) -> r2g_mlua::prelude::LuaResult<r2g_mlua::prelude::LuaValue> {
                    use crate::instance::IObjectComponent;
                    self.#self_component_name.read().unwrap().lua_get(self, lua, &name)
                        #(#hierarchy_based_component_list)*
                        .unwrap_or_else(|| Error(r2g_mlua::prelude::LuaError::RuntimeError(format!("Property {} does not exist on class {}", name, self.get_class_name()).into())))
                }
                fn lua_set(&self, lua: &r2g_mlua::Lua, name: String, val: r2g_mlua::prelude::LuaValue) -> r2g_mlua::prelude::LuaResult<()> {
                    use crate::instance::IObjectComponent;
                    self.#self_component_name.write().unwrap().lua_set(self, lua, &name, val)
                        #(#hierarchy_based_component_list)*
                        .unwrap_or_else(|| Error(r2g_mlua::prelude::LuaError::RuntimeError(format!("Property {} does not exist on class {}", name, self.get_class_name()).into())))
                }
            };
        }
        let hierarchy_slice = &hierarchy[..hierarchy.len() - 1];
        iobject_impl_block = quote! {
            impl crate::core::IObject for #class_name {
                fn get_class_name(&self) -> &'static str {
                    stringify!(#class_name)
                }
                fn is_a(&self, class_name: &String) -> bool {
                    match class_name.as_str() {
                        stringify!(#class_name)
                        #( | stringify!(#hierarchy_slice))*
                        | "Object"
                        => true,
                        _ => false
                    }
                }
                #signals_defs_block
                #lua_get_set_block
            }
        }
    }
    let obj_component_impl = {
        let mut error_flag = None;
        // in order: get, set, init, clone
        let properties: Vec<(TokenStream, TokenStream, TokenStream)> = object_info.properties.iter().filter_map(|x|
            match x {
                ObjectPropertyInfo::Rust { name, default, .. } => {
                    Some((
                        quote! {},
                        quote! {},
                        default.as_ref().map_or_else(|| quote! { #name: core::default::Default::default() }, |x| quote! {
                            #name: #x
                        })
                    ))
                }
                ObjectPropertyInfo::Transparent(info) => {
                    let name = LitStr::new(info.name.as_str(), info.span);
                    if info.get.is_none() || (info.set.is_none() && !info.readonly) {
                        error_flag = Some(syn::Error::new(info.span, "Transparent properties must have both get and set methods defined"));
                        return None;
                    }
                    let get = info.get.as_ref().unwrap();
                    let mut get_set = if info.readonly {
                        (
                            quote! {
                                #name => Some(self.#get (ptr, lua))
                            },
                            quote! {
                                #name => Some(Err(r2g_mlua::prelude::LuaError::RuntimeError(format!("cannot set readonly property {}", stringify!(#name)))))
                            },
                        )
                    } else {
                        let set = info.set.as_ref().unwrap();
                        (
                            quote! {
                                #name => Some(self.#get (ptr, lua))
                            },
                            quote! {
                                #name => Some(self.#set (ptr, lua, value.clone()))
                            },
                        )
                    };
                    if info.security_context != SecurityContext::NONE {
                        let (get, set) = get_set;
                        let context = info.security_context;
                        let l = |x| quote! {
                                if crate::core::get_current_identity(lua).map(|x| x.get_security_contexts()).unwrap_or_default().has(#context) {
                                    #x
                                } else {
                                    Err(r2g_mlua::prelude::LuaError::RuntimeError(format!("security violation in accessing property {}", stringify!(#name))))
                                }
                            };
                        get_set = (
                            l(get),
                            l(set)
                        );
                    }
                    Some((
                        get_set.0,
                        get_set.1,
                        TokenStream::new()
                    ))
                },
                ObjectPropertyInfo::Lua { name: property_name, ty, default, info } => {
                    let name = Ident::new(info.name.as_str(), info.span);
                    let mut get = if let Some(getter) = info.get.as_ref() {
                        quote! {
                            stringify!(#name) => Some(self.#getter(ptr, lua))
                        }
                    } else {
                        quote! {
                            stringify!(#name) => {
                                use r2g_mlua::IntoLua;
                                Some(self.#property_name.clone().into_lua(lua))
                            }
                        }
                    };
                    let mut set = if info.readonly {
                        quote! {
                            stringify!(#name) => Some(Err(r2g_mlua::prelude::LuaError::RuntimeError(format!("cannot set readonly property {}", stringify!(#name)))))
                        }
                    } else if let Some(setter) = info.set.as_ref() {
                        quote! {
                            stringify!(#name) => Some(self.#setter(ptr, lua, value.clone()))
                        }
                    } else {
                        quote! {
                            stringify!(#name) => {
                                self.#property_name = r2g_mlua::FromLua::from_lua(value.clone(), lua)?;
                                Some(Ok(()))
                            }
                        }
                    };
                    if info.security_context != SecurityContext::NONE {
                        let context = info.security_context;
                        let l = |x| quote! {
                                if crate::core::get_current_identity(lua).map(|x| x.get_security_contexts()).unwrap_or_default().has(#context) {
                                    #x
                                } else {
                                    Some(Err(r2g_mlua::prelude::LuaError::RuntimeError(format!("security violation in accessing property {}", stringify!(#name)))))
                                }
                            };
                        get = l(get);
                        set = l(set);
                    }
                    Some((
                        get,
                        set,
                        default.as_ref().map_or_else(|| quote! { #name: core::default::Default::default() }, |x| quote! {
                            #name: #x
                        })
                    ))
                }
            }
        ).collect();
        if let Some(e) = error_flag {
            return e.into_compile_error().into();
        }
        let mut getters = Vec::with_capacity(properties.len());
        let mut setters = Vec::with_capacity(properties.len());
        let mut defaults = Vec::with_capacity(properties.len());
        for (get, set, default) in properties {
            if !get.is_empty() {
                getters.push(get);
            }
            if !set.is_empty() {
                setters.push(set);
            }
            if !default.is_empty() {
                defaults.push(default);
            }
        }
        let init_code = if first_flags & ObjectMask::ComponentInit != ObjectMask::None {
            quote! {
                Self::init(lua, metadata)
            }
        } else {
            quote! {
                Ok(Self {
                    #(#defaults,)*
                })
            }
        };
        quote! {
            #[allow(unused_variables)]
            impl crate::core::IObjectComponent for #component_class_name {
                fn lua_get(
                    self: &mut crate::core::RwLockReadGuard<'_, Self>,
                    ptr: &crate::core::DynObject,
                    lua: &r2g_mlua::prelude::Lua,
                    key: &String,
                ) -> Option<r2g_mlua::prelude::LuaResult<r2g_mlua::prelude::LuaValue>> {
                    match key.as_str() {
                        #(#getters,)*
                        _ => self.get_lua_method(ptr, lua, key).map(|x| x.into_lua(lua))
                    }
                }
                fn lua_set(
                    self: &mut crate::core::RwLockWriteGuard<'_, Self>,
                    ptr: &crate::core::DynObject,
                    lua: &r2g_mlua::prelude::Lua,
                    key: &String,
                    value: &r2g_mlua::prelude::LuaValue,
                ) -> Option<r2g_mlua::prelude::LuaResult<()>> {
                    match key.as_str() {
                        #(#setters,)*
                        _ => None
                    }
                }
                fn new(metadata: &crate::core::ObjectCreationMetadata) -> LuaResult<Self> {
                    #init_code
                }
            }
        }
    };
    let object_init = if first_flags & ObjectMask::Abstract != ObjectMask::None {
        quote! {}
    } else {
        let components = hierarchy[..hierarchy.len() - 1]
            .iter()
            .map(|x| {
                let snake_case = camel_case_to_snake_case(x.to_string().as_str());
                let component_name = Ident::new(&format!("{}_component", snake_case), x.span());
                let component_type = Ident::new(&format!("{}Component", x.to_string()), x.span());
                quote! {
                    #component_name: crate::core::RwLock::new_with_flag_auto(crate::instance::#component_type::new(lua, &metadata)?),
                }
            })
            .collect::<Vec<_>>();
        let mut init_func = if first_flags & ObjectMask::RequiresInit != ObjectMask::None {
            quote! {
                obj.init();
            }
        } else {
            quote! {}
        };
        let construction_block = if hierarchy
            .get(hierarchy.len() - 2)
            .map(|x| x.to_string() == "Instance")
            .unwrap_or(false)
        {
            quote! {
                let mut i = Self {
                    #(#components,)*
                };
                crate::core::DynInstance::submit_metadata(&mut i, metadata);
                i
            }
        } else {
            quote! {
                Self {
                    #(#components,)*
                }
            }
        };
        quote! {
            impl #class_name {
                pub fn new(lua: &Lua) -> crate::core::Irc::<Self> {
                    use crate::core::Irc;
                    let obj = Irc::<Self>::new_cyclic_fallable(|x| {
                        let metadata = crate::core::ObjectCreationMetadata::new(stringify!(#class_name), x.cast_to_object());

                        #construction_block
                    });

                    #init_func

                    obj
                }
            }
        }
    };
    quote! {
        #[derive(Debug)]
        #struct_block
        #[derive(Debug)]
        #vis #struct_block_with_components
        #inheritance_base_block
        #vis #trait_block
        #(#hierarchy_inherit_block)*
        #iobject_impl_block
        #obj_component_impl
        #object_init
        #object_info
    }
    .into()
}

#[proc_macro_attribute]
pub fn instance_class(
    arguments: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ts_clone = ts.clone();
    let struct_block: ItemStruct = parse_macro_input!(ts_clone);
    let args = parse_macro_input!(arguments as AttrArguments);

    let mut hierarchy;
    if let Some(h) = args.get_named_arg("hierarchy") {
        if let Ok(expr) = h.value.clone().try_into() {
            if let syn::Expr::Array(arr) = expr {
                let mut v = Vec::new();
                for e in arr.elems.iter() {
                    if let syn::Expr::Path(p) = e {
                        if let Some(i) = p.path.get_ident() {
                            v.push(i.clone());
                        } else {
                            return syn::Error::new(p.path.span(), "not an ident")
                                .into_compile_error()
                                .into();
                        }
                    } else {
                        return syn::Error::new(e.span(), "not a path")
                            .into_compile_error()
                            .into();
                    }
                }
                hierarchy = v;
            } else {
                return syn::Error::new(h.value.span(), "not an array")
                    .into_compile_error()
                    .into();
            }
        } else {
            return syn::Error::new(h.value.span(), "not an expression")
                .into_compile_error()
                .into();
        }
    } else {
        hierarchy = vec![];
    }
    hierarchy.push(Ident::new("Instance", Span::call_site()));
    let mut instance_flags = InstanceMask::None;

    {
        if args.has_unnamed_arg("non_constructible") {
            instance_flags |= InstanceMask::NonConstructible;
        }
        if args.has_unnamed_arg("no_clone") {
            instance_flags |= InstanceMask::NoClone;
        }
        if args.has_unnamed_arg("parent_change_locked") {
            instance_flags |= InstanceMask::ParentChangeLocked;
        }
        if args.has_unnamed_arg("no_destruction") {
            instance_flags |= InstanceMask::NoDestruction;
        }
    }
    let mut obj_mask = ObjectMask::None;
    let arguments = {
        let abstract_kw = if args.has_unnamed_arg("abstract") {
            obj_mask |= ObjectMask::Abstract;
            quote! { "abstract", }
        } else {
            quote! {}
        };
        let requires_init_kw = if args.has_unnamed_arg("requires_init") {
            obj_mask |= ObjectMask::RequiresInit;
            quote! { "requires_init", }
        } else {
            quote! {}
        };
        let component_requires_init_kw = if args.has_unnamed_arg("component_requires_init") {
            obj_mask |= ObjectMask::ComponentInit;
            quote! { "component_requires_init", }
        } else {
            quote! {}
        };

        quote! {
            hierarchy= [#(#hierarchy,)*],#abstract_kw #requires_init_kw #component_requires_init_kw
        }
    };

    let class_name = struct_block.ident.clone();
    let trait_name = Ident::new(&format!("I{}", class_name.to_string()), class_name.span());
    let component_name = Ident::new(
        &format!("{}Component", class_name.to_string()),
        class_name.span(),
    );
    let snake_case = Ident::new(
        camel_case_to_snake_case(class_name.to_string().as_str()).as_str(),
        class_name.span(),
    );

    let info = {
        let mut obj_class_info = PartialObjectClassInfo::new(&class_name, Some(&hierarchy));
        obj_class_info.populate_table(obj_mask, &struct_block);
        obj_class_info
            .hierarchy
            .as_mut()
            .map(|v| v.retain(|x| x.to_string() != "Instance"));
        PartialInstanceClassInfo::from((obj_class_info, instance_flags))
    };
    let instance_class_info = info.export();

    let instance_component_impl = {
        if instance_flags & InstanceMask::NoClone == InstanceMask::None {
            let clones: Vec<_> = info
                .properties
                .iter()
                .filter_map(|x| match x {
                    ObjectPropertyInfo::Transparent(_) => None,
                    ObjectPropertyInfo::Lua { name, .. }
                    | ObjectPropertyInfo::Rust { name, .. } => Some(quote! {
                        #name: self.#name.clone()
                    }),
                })
                .collect();
            quote! {
                impl crate::core::IInstanceComponent for #component_name {
                    fn clone(
                        self: &RwLockReadGuard<'_, Self>,
                        lua: &Lua,
                        metadata: &crate::core::ObjectCreationMetadata
                    ) -> LuaResult<Self> {
                        Ok(Self {
                            #(#clones,)*
                        })
                    }
                }
            }
        } else {
            quote! {
                impl crate::core::IInstanceComponent for #component_name {
                    fn clone(
                        self: &RwLockReadGuard<'_, Self>,
                        lua: &Lua,
                        metadata: &ObjectCreationMetadata,
                    ) -> LuaResult<Self> {
                        Err(r2g_mlua::prelude::LuaError::RuntimeError(
                            format!("Cannot clone instance of class {}", stringify!(#class_name)).into(),
                        ))
                    }
                }
            }
        }
    };

    let clone_impl = if instance_flags & InstanceMask::NoClone == InstanceMask::None {
        let components = hierarchy
            .iter()
            .map(|x| {
                let snake_case = camel_case_to_snake_case(x.to_string().as_str());
                let component_name = Ident::new(&format!("{}_component", snake_case), x.span());
                quote! {
                    #component_name: crate::core::RwLock::new_with_flag_auto(self.#component_name.read().unwrap().clone(lua, &metadata)?)
                }
            })
            .collect::<Vec<_>>();
        let component_name = Ident::new(
            &format!("{}_component", snake_case.to_string().as_str()),
            class_name.span(),
        );
        quote! {
            use crate::core::IInstanceComponent;
            use crate::core::Irc;
            let obj: r2g_mlua::prelude::LuaResult<Irc<Self>> = Irc::<Self>::new_cyclic_fallable(|x| {
                let metadata = crate::core::ObjectCreationMetadata::new(stringify!(#class_name), x.cast_to_object());
                let mut i = Self {
                    #component_name: crate::core::RwLock::new_with_flag_auto(self.#component_name.read().unwrap().clone(lua, &metadata)?),
                    #(#components,)*
                };
                crate::core::DynInstance::submit_metadata(&mut i, metadata);
                Ok(i)
            });
            Ok(obj?.cast_from_sized::<crate::core::DynInstance>().unwrap())
        }
    } else {
        quote! {
            Err(r2g_mlua::prelude::LuaError::RuntimeError(
                format!("Cannot clone instance of class {}", stringify!(#class_name)).into(),
            ))
        }
    };

    let object_class_def: proc_macro2::TokenStream = object_class(arguments.into(), ts).into();
    quote! {
        #object_class_def
        #instance_class_info
        #instance_component_impl

        impl crate::core::IInstance for #class_name {
            fn get_instance_component(&self) -> crate::core::RwLockReadGuard<'_, crate::instance::InstanceComponent> {
                self.instance_component.read().unwrap()
            }
            fn get_instance_component_mut(&self) -> crate::core::RwLockWriteGuard<'_, crate::instance::InstanceComponent> {
                self.instance_component.write().unwrap()
            }
            fn clone_instance(&self, lua: &r2g_mlua::Lua) -> r2g_mlua::prelude::LuaResult<crate::core::ManagedInstance> {
                #clone_impl
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn object_methods(
    arguments: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut impl_block: ItemImpl = parse_macro_input!(ts);

    let class_name = if let syn::Type::Path(path) = impl_block.self_ty.as_ref() {
        if path.path.segments.len() == 1 {
            path.path.segments[0].ident.clone()
        } else {
            return syn::Error::new(impl_block.self_ty.span(), "expected ident")
                .into_compile_error()
                .into();
        }
    } else {
        return syn::Error::new(impl_block.self_ty.span(), "expected ident")
            .into_compile_error()
            .into();
    };

    let mut lua_funcs = Vec::new();
    let items = std::mem::take(&mut impl_block.items)
        .into_iter()
        .filter_map(|x| match x {
            ImplItem::Fn(mut func) => {
                if let Some(_) = func
                    .attrs
                    .iter()
                    .filter(|a| a.path().is_ident("override"))
                    .next()
                {
                    // Function is virtual (overriding)
                    /*let class_overriding: Ident = if let Ok(res) = s.parse_args() {
                        res
                    } else {
                        return syn::Error::new(s.span(), "expected ident after override")
                            .into_compile_error()
                            .into();
                    };*/
                    func.sig.ident = Ident::new(
                        &format!("_{}_impl", func.sig.ident.to_string()),
                        func.sig.ident.span(),
                    );
                    if func.sig.asyncness.is_some() {
                        return syn::Error::new(func.sig.span(), "virtual methods cannot be async")
                            .into_compile_error()
                            .into();
                    }
                    if func.sig.generics.params.len() > 0 {
                        return syn::Error::new(
                            func.sig.span(),
                            "virtual methods cannot have generic parameters",
                        )
                        .into_compile_error()
                        .into();
                    }
                } else if let Some(lua_method) = func
                    .attrs
                    .iter()
                    .filter(|x| x.path().is_ident("lua_method"))
                    .next()
                {
                    let args: AttrArguments = if let Ok(res) = lua_method.parse_args() {
                        res
                    } else {
                        return syn::Error::new(
                            lua_method.span(),
                            "expected arguments after lua_method",
                        )
                        .into_compile_error()
                        .into();
                    };
                    let name = args
                        .get_named_arg("name")
                        .map(|x| &x.value)
                        .and_then(|x| match x {
                            AttrArgValue::Expr(syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(s),
                                ..
                            })) => Some(s.value()),
                            _ => None,
                        })
                        .unwrap_or(snake_case_to_camel_case(
                            func.sig.ident.to_string().as_str(),
                        ));
                    if func.sig.asyncness.is_some() {
                        return syn::Error::new(func.sig.span(), "Lua methods cannot be async")
                            .into_compile_error()
                            .into();
                    }
                    if func.sig.generics.params.len() > 0 {
                        return syn::Error::new(
                            func.sig.span(),
                            "virtual methods cannot have generic parameters",
                        )
                        .into_compile_error()
                        .into();
                    }
                    let security_context = match args
                        .get_named_arg("security_context")
                        .map(|x| match &x.value {
                            AttrArgValue::Expr(syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(s),
                                ..
                            })) => Ok(match s.value().as_str() {
                                "None" => SecurityContext::NONE,
                                "LocalUser" => SecurityContext::LOCAL_USER,
                                "Plugin" => SecurityContext::PLUGIN,
                                "Roblox" => SecurityContext::ROBLOX,
                                "RobloxScript" => SecurityContext::ROBLOX_SCRIPT,
                                "TestLocalUser" => SecurityContext::TEST_LOCAL_USER,
                                "WritePlayer" => SecurityContext::WRITE_PLAYER,
                                _ => {
                                    return Err(syn::Error::new(
                                        s.span(),
                                        "invalid security context",
                                    ))
                                }
                            }),
                            _ => Err(syn::Error::new(
                                x.span(),
                                "expected string literal for security_context",
                            )),
                        })
                        .unwrap_or(Ok(SecurityContext::NONE))
                    {
                        Ok(context) => context,
                        Err(e) => return e.into_compile_error().into(),
                    };
                    lua_funcs.push(LuaFunctionInfo {
                        lua_name: name,
                        security_context,
                        sig: func.sig.clone(),
                        span: func.span(),
                    });
                }
                Some(func.to_token_stream())
            }
            ImplItem::Const(const_item) => Some(const_item.to_token_stream()),
            ImplItem::Type(type_item) => Some(type_item.to_token_stream()),
            ImplItem::Macro(macro_item) => Some(macro_item.to_token_stream()),
            _ => {
                return syn::Error::new(x.span(), "Unexpected item in object_methods")
                    .into_compile_error()
                    .into();
            }
        })
        .collect::<Vec<_>>();
    let exported_funcs = lua_funcs
        .iter()
        .map(|f| {
            let lua_name = Ident::new(f.lua_name.as_str(), Span::call_site());
            let security_context = f.security_context;
            let sig_name = &f.sig.ident;
            let sig_ret_type = if let syn::ReturnType::Type(_, ty) = &f.sig.output {
                Ident::new(ty.to_token_stream().to_string().as_str(), Span::call_site())
            } else {
                Ident::new("()", Span::call_site())
            };
            let sig_args = f
                .sig
                .inputs
                .iter()
                .map(|arg| Ident::new(arg.to_token_stream().to_string().as_str(), arg.span()))
                .collect::<Vec<_>>();
            quote! {
                crate::core::LuaFunctionInfo {
                    lua_name: #lua_name,
                    security_context: #security_context,
                    sig: crate::core::LuaFunctionSignature {
                        name: stringify!(#sig_name),
                        args: &[#(#sig_args,)*],
                        ret: stringify!(#sig_ret_type),
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let component_class_name = Ident::new(
        &format!("{}Component", class_name.to_string()),
        class_name.span(),
    );
    quote! {
        impl #class_name {
            #(#items)*
        }
        impl #component_class_name {
            fn get_lua_method(
                self: &mut crate::core::RwLockReadGuard<'_, Self>,
                ptr_self: &crate::core::DynObject,
                lua: &r2g_mlua::Lua,
                name: &String
            ) -> Option<r2g_mlua::prelude::LuaFunction> {
                match name.as_str() {
                    _ => None
                }
            }
        }
        impl crate::core::ObjectClass for #class_name {
            const OBJECT_CLASS_INFO: crate::core::ObjectClassInfo = crate::core::ObjectClassInfo {
                partial_info: <Self as crate::core::PartialObjectClass>::PARTIAL_OBJECT_CLASS_INFO,
                method_list: &[
                    #(#exported_funcs,)*
                ]
            };
        }
    }
    .into()
}
