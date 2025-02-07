#![feature(if_let_guard)]

use convert_case::Casing;
use parse::{
    parse_lua_fn_attr, Instance, InstanceConfig, InstanceConfigAttr, LuaFunctionData,
    LuaPropertyData,
};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, Error, ExprArray,
    Field, Ident, ItemEnum, ItemImpl, LitStr, Path, PathSegment, Token, TraitBound, TypeParamBound,
    TypeTraitObject,
};
use utils::camel_case_to_snake_case;

mod parse;
mod utils;

macro_rules! error_cattr {
    ($span:expr, $err:literal) => {
        syn::Error::new($span, $err).into_compile_error()
    };
}

#[proc_macro_attribute]
pub fn instance(
    item: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item: Punctuated<InstanceConfigAttr, Token![,]> =
        match Punctuated::parse_terminated.parse(item) {
            Ok(s) => s,
            Err(e) => return e.into_compile_error().into(),
        };

    // convert punct to conf
    let mut no_clone = None;
    let mut parent_locked = None;
    let mut hierarchy: Option<Vec<syn::Path>> = None;
    let mut custom_new = None;
    let mut requires_init = None;

    for ca in item {
        match ca {
            InstanceConfigAttr::NoClone(b, _, span) => {
                if no_clone.is_none() {
                    no_clone = Some(b)
                } else {
                    return error_cattr!(span, "`no_clone` specified twice").into();
                }
            }
            InstanceConfigAttr::ParentLocked(b, _eq, span) => {
                if parent_locked.is_none() {
                    parent_locked = Some(b)
                } else {
                    return error_cattr!(span, "`parent_locked` specified twice").into();
                }
            }
            InstanceConfigAttr::Hierarchy(_eq, _bracket, punctuated, span) => {
                if hierarchy.is_none() {
                    hierarchy = Some(punctuated.into_iter().collect())
                } else {
                    return error_cattr!(span, "`hierarchy` specified twice").into();
                }
            }
            InstanceConfigAttr::CustomNew(b, _eq, span) => {
                if custom_new.is_none() {
                    custom_new = Some(b)
                } else {
                    return error_cattr!(span, "`custom_new` specified twice").into();
                }
            }
            InstanceConfigAttr::RequiresInit(b, _eq, span) => {
                if requires_init.is_none() {
                    requires_init = Some(b)
                } else {
                    return error_cattr!(span, "`requires_init` specified twice").into();
                }
            }
        }
    }

    let ic = InstanceConfig {
        no_clone: no_clone.unwrap_or(false),
        parent_locked: parent_locked.unwrap_or(false),
        hierarchy: hierarchy.unwrap_or(vec![]),
        custom_new: custom_new.unwrap_or(false),
        requires_init: requires_init.unwrap_or(false),
    };
    let ts1 = ts.clone(); // temporary
    let mut inst: Instance = parse_macro_input!(ts1);

    let mut lua_fns: Vec<LuaFunctionData> = vec![];
    let mut taken = vec![];
    let mut i = 0;
    while i < inst.attrs.len() {
        if inst.attrs[i].path().is_ident("method") {
            taken.push(inst.attrs.swap_remove(i)); // Swap and remove the matching item
        } else {
            i += 1; // Only increment if no removal
        }
    }

    for attr in taken {
        lua_fns.push(match parse_lua_fn_attr(attr) {
            Ok(s) => s,
            Err(e) => return e.into_compile_error().into(),
        });
    }

    let mut rust_fields: Vec<Field> = vec![];
    let mut lua_fields: Vec<LuaPropertyData> = vec![];

    for i in inst.contents.named {
        match i {
            parse::InstanceContent::RustField { rust_field } => rust_fields.push(rust_field),
            parse::InstanceContent::LuaField {
                lua_field,
                rust_field,
            } => {
                if !lua_field.transparent {
                    rust_fields.push(rust_field);
                }
                lua_fields.push(lua_field);
            }
        }
    }

    let (attr, vis, struct_token, gens, ident) = (
        inst.attrs,
        inst.vis,
        inst.struct_token,
        inst.generics,
        inst.ident,
    );

    let component_name = Ident::new(&(ident.to_string() + "Component"), ident.span());
    let trait_name = Ident::new(&("I".to_owned() + &ident.to_string()), ident.span());

    let snake = ident.to_string().to_case(convert_case::Case::Snake);
    let snake_id = Ident::new(&snake, ident.span());
    let mut component_get_name = String::from("get_");
    component_get_name.push_str(&snake);
    component_get_name.push_str("_component");

    let mut component_get_mut_name = String::from("get_");
    component_get_mut_name.push_str(&snake);
    component_get_mut_name.push_str("_component_mut");

    let (cgn, cgmn) = (
        Ident::new(&component_get_name, ident.span()),
        Ident::new(&component_get_mut_name, ident.span()),
    );

    let inherited_names: Vec<LitStr> = ic
        .hierarchy
        .iter()
        .map(|i| LitStr::new(&i.segments.last().unwrap().ident.to_string(), i.span()))
        .collect();

    let inherited: Vec<syn::Path> = ic
        .hierarchy
        .into_iter()
        .into_iter()
        .map(|i| syn::Path {
            leading_colon: i.leading_colon,
            segments: {
                let mut punct: Punctuated<PathSegment, Token![::]> = Punctuated::new();

                let len = i.segments.len();

                for (i, mut element) in i.segments.into_iter().enumerate() {
                    if i < len - 1 {
                        punct.push(element);
                    } else {
                        element.ident = Ident::new(
                            &("I".to_owned() + &element.ident.to_string()),
                            element.ident.span(),
                        );
                        punct.push(element);
                    }
                }

                punct
            },
        })
        .collect();

    let s = LitStr::new(&ident.to_string(), ident.span());

    let iinstance_lua_get = {
        // todo: add more features (e.g. security enforcement)
        let mut quotes: Vec<proc_macro2::TokenStream> = vec![];
        for f in &lua_fields {
            if f.transparent {
                if let Some(get) = &f.get {
                    let name = LitStr::new(&f.name, Span::call_site());
                    quotes.push(quote! {
                        #name => Some(lua_getter!(lua, #get(ptr, lua)))
                    });
                } else {
                    return syn::Error::new(f.span, "has no getter despite being transparent")
                        .into_compile_error()
                        .into();
                }
            } else if let Some(get) = &f.get {
                let name = LitStr::new(&f.name, Span::call_site());
                quotes.push(quote! {
                    #name => Some(lua_getter!(lua, #get(ptr, lua)))
                });
            } else {
                let lua_name = LitStr::new(&f.name, Span::call_site());
                let rust_name = &f.rust_name;

                quotes.push(quote! {
                    #lua_name => Some(lua_getter!(clone, lua, self.#rust_name))
                });
            }
        }

        quotes
    };

    let field_news = {
        // todo: finish this
        let mut quotes: Vec<proc_macro2::TokenStream> = vec![];

        for f in &lua_fields {
            if let Some(s) = &f.default {
                let n = &f.rust_name;
                if let Some(s) = s {
                    quotes.push(quote! {
                        #n: #s
                    });
                } else {
                    quotes.push(quote! {
                        #n: Default::default()
                    });
                }
            } else {
                return syn::Error::new(
                    f.span,
                    "default impl required for field (use Option if you don't have one)",
                )
                .into_compile_error()
                .into();
            }
        }

        quotes
    };

    let iinstance_lua_set = {
        // todo: add more features (e.g. security enforcement)
        let mut quotes: Vec<proc_macro2::TokenStream> = vec![];

        for f in lua_fields {
            if f.readonly {
                let name = LitStr::new(&f.name, Span::call_site());
                quotes.push(quote! {
                    #name => Some(Err(r2g_mlua::prelude::LuaError::RuntimeError("Cannot set read only property.".into())))
                });
            }
        }

        quotes
    };

    quote! {
        //static DEBUG: &str = #ls;
        // static IC: &str = #moar;
        // static RECEIVED_CODE: &str = #code;
        // static FNS: &str = #lua_fns;
        #(#attr)* #vis #struct_token #gens #component_name {
            #(#rust_fields),*
        }

        impl crate::instance::IInstanceComponent for #component_name {
            fn new(_ptr: crate::instance::WeakManagedInstance, _class_name: &'static str) -> Self {
                todo!("implement `new` *cleanly*. somehow. who knows how. cuz rust fields idk how to implement default/new for them here (lua fields are handled)")
            }

            fn clone(self: &crate::core::RwLockReadGuard<'_, Self>, _: &r2g_mlua::Lua, _: &crate::instance::WeakManagedInstance) -> r2g_mlua::prelude::LuaResult<Self> {
                Err(r2g_mlua::prelude::LuaError::RuntimeError("Cannot clone DataModelComponent".into()))
            }

            fn lua_get(self: &mut crate::core::RwLockReadGuard<'_, Self>, ptr: &crate::instance::DynInstance, lua: &r2g_mlua::Lua, key: &String) -> Option<r2g_mlua::prelude::LuaResult<r2g_mlua::prelude::LuaValue>> {
                use crate::core::lua_macros::lua_getter;
                use crate::core::inheritance_cast_to;
                use crate::core::lua_macros::lua_invalid_argument;
                use r2g_mlua::prelude::IntoLua;
                match key.as_str() {
                    #(#iinstance_lua_get),*,
                    _ => None
                }
            }

            fn lua_set(self: &mut crate::core::RwLockWriteGuard<'_, Self>, _ptr: &crate::instance::DynInstance, _lua: &r2g_mlua::Lua, key: &String, _value: &r2g_mlua::prelude::LuaValue) -> Option<r2g_mlua::prelude::LuaResult<()>> {
                use r2g_mlua::prelude::IntoLua;
                match key.as_str() {
                    #(#iinstance_lua_set),*,
                    _ => None
                }
            }
        }

        trait #trait_name {
            fn #cgn(&self) -> crate::core::RwLockReadGuard<'_, #component_name>;
            fn #cgmn(&self) -> crate::core::RwLockWriteGuard<'_, #component_name>;
        }

        struct #ident {
            // base
            instance: crate::core::RwLock<crate::instance::InstanceComponent>,
            // all elements in hierarchy
            service_provider: crate::core::RwLock<crate::instance::ServiceProviderComponent>,
            // self
            #snake_id: crate::core::RwLock<#component_name>,
        }

        impl crate::core::InheritanceBase for #ident {
            fn inheritance_table(&self) -> crate::core::InheritanceTable {
                crate::core::InheritanceTableBuilder::new()
                    .insert_type::<#ident, dyn crate::instance::IObject>(|x| x, |x| x)
                    .insert_type::<#ident, crate::instance::DynInstance>(|x| x, |x| x)
                    #(.insert_type::<#ident, dyn #inherited>(|x| x, |x| x))*
                    .insert_type::<#ident, dyn #trait_name>(|x| x, |x| x)
                    .output()
            }
        }

        impl crate::instance::IObject for #ident {
            fn is_a(&self, class_name: &String) -> bool {
                match class_name.as_str() {
                    "DataModel" => true,
                    //"ServiceProvider" => true,
                    "Instance" => true,
                    "Object" => true,
                    #(#inherited_names => true),*,
                    #s => true,
                    _ => false
                }
            }
            fn lua_get(&self, lua: &r2g_mlua::Lua, name: String) -> r2g_mlua::prelude::LuaResult<r2g_mlua::prelude::LuaValue> {
                use crate::instance::IInstanceComponent;
                self.#snake_id.read().unwrap().lua_get(self, lua, &name)
                    .or_else(|| self.service_provider.read().unwrap().lua_get(self, lua, &name))
                    .unwrap_or_else(|| self.instance.read().unwrap().lua_get(lua, &name))
            }
            fn get_changed_signal(&self) -> crate::userdata::ManagedRBXScriptSignal {
                use crate::instance::IInstance;
                self.get_instance_component().changed.clone()
            }
            fn get_property_changed_signal(&self, property: String) -> crate::userdata::ManagedRBXScriptSignal {
                use crate::instance::IInstance;
                self.get_instance_component().get_property_changed_signal(property).unwrap()
            }
            fn get_class_name(&self) -> &'static str { #s }
        }

        impl #trait_name for #ident {
            fn #cgn(&self) -> crate::core::RwLockReadGuard<'_, #component_name> {
                self.#snake_id.read().unwrap()
            }

            fn #cgmn(&self) -> crate::core::RwLockWriteGuard<'_, #component_name> {
                self.#snake_id.write().unwrap()
            }
        }

        impl crate::instance::IInstance for #ident {
            fn get_instance_component(&self) -> crate::core::RwLockReadGuard<crate::instance::InstanceComponent> {
                self.instance.read().unwrap()
            }

            fn get_instance_component_mut(&self) -> crate::core::RwLockWriteGuard<crate::instance::InstanceComponent> {
                self.instance.write().unwrap()
            }

            fn lua_set(&self, lua: &r2g_mlua::Lua, name: String, val: r2g_mlua::prelude::LuaValue) -> r2g_mlua::prelude::LuaResult<()> {
                use crate::instance::IInstanceComponent;
                self.#snake_id.write().unwrap().lua_set(self, lua, &name, &val)
                    .or_else(|| self.service_provider.write().unwrap().lua_set(self, lua, &name, &val))
                    .unwrap_or_else(|| self.instance.write().unwrap().lua_set(lua, &name, val))
            }

            fn clone_instance(&self, _: &r2g_mlua::Lua) -> r2g_mlua::prelude::LuaResult<crate::instance::ManagedInstance> {
                todo!("implement this cleanly too (same issue applies, but if no_clone this is optional)")
            }
        }

        impl crate::instance::IServiceProvider for #ident {
            fn get_service_provider_component(&self) -> crate::core::RwLockReadGuard<crate::instance::ServiceProviderComponent> {
                self.service_provider.read().unwrap()
            }

            fn get_service_provider_component_mut(&self) -> crate::core::RwLockWriteGuard<crate::instance::ServiceProviderComponent> {
                self.service_provider.write().unwrap()
            }

            fn get_service(&self, service_name: String) -> r2g_mlua::prelude::LuaResult<crate::instance::ManagedInstance> {
                self.find_service(service_name)
                    .and_then(|x|
                        x.ok_or_else(|| r2g_mlua::prelude::LuaError::RuntimeError("Service not found".into()))
                    )
            }

            fn find_service(&self, service_name: String) -> r2g_mlua::prelude::LuaResult<Option<crate::instance::ManagedInstance>> {
                crate::instance::DynInstance::find_first_child_of_class(self, service_name)
            }
        }
    }.into_token_stream().into()
}

// #[proc_macro_attribute]
// pub fn property(_item: proc_macro::TokenStream, ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     ts
// }

#[proc_macro_attribute]
pub fn methods(
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

#[proc_macro_attribute]
pub fn lua_enum(
    _item: proc_macro::TokenStream,
    ts: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let enum_block: ItemEnum = parse_macro_input!(ts);
    let name = enum_block.ident.clone();
    let vis = enum_block.vis.clone();

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

    let variant_quotes_names: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|(variant_name, _value)| {
            let variant_name = Ident::new(variant_name, Span::call_site());
            quote! {
                Self::#variant_name => concat!(stringify!(#name), ".", stringify!(#variant_name))
            }
        })
        .collect();
    let variant_quotes_names_only: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|(variant_name, _value)| {
            let variant_name = Ident::new(variant_name, Span::call_site());
            quote! {
                Self::#variant_name => stringify!(#variant_name)
            }
        })
        .collect();

    let variant_fields: Vec<proc_macro2::TokenStream> = variants
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

    let modules: Vec<proc_macro2::TokenStream> = enums
        .iter()
        .map(|x| {
            let ident = Ident::new(&camel_case_to_snake_case(x.to_string().as_str()), x.span());
            quote! {
                mod #ident;
            }
        })
        .collect();

    let enum_use: Vec<proc_macro2::TokenStream> = enums
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

    let enum_fields: Vec<proc_macro2::TokenStream> = enums
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
