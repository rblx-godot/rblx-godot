use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{__private::ToTokens, parse::Parser, spanned::Spanned, visit::Visit, Attribute, Data, Error, Ident, LitBool, LitStr};
use syn::{parse_quote, Field};

enum SecurityContext {
    None,
    PluginSecurity,
    LocalUserSecurity,
    RobloxScriptSecurity
}

struct ParsedProperty {
    name: String,
    readonly: Option<bool>, // implicit
    get: Option<String>,
    set: Option<String>,
    security_context: Option<SecurityContext>,

    field: Field
}

impl ParsedProperty {
    fn new(field: Field) -> Self {
        Self { name: String::new(), readonly: None, get: Default::default(), set: Default::default(), security_context: None, field }
    }
}

// struct PropertyParser;

// impl Parser for PropertyParser {
//     type Output = ParsedProperty;

//     fn parse2(self, tokens: proc_macro2::TokenStream) -> syn::Result<Self::Output> {
//         for tt in tokens {
            
//         }
//         todo!()
//     }
// }

struct InstanceDeriveVisitor {
    errors: proc_macro2::TokenStream,
    errored: bool,
    prop: Vec<ParsedProperty>
}

impl Visit<'_> for InstanceDeriveVisitor {
    fn visit_field(&mut self, field: &syn::Field) {
        let attrs: Vec<&Attribute> = field.attrs.iter().filter(|attr| attr.meta.path().is_ident("property")).collect();
        if attrs.len() == 1 {
            let attr = attrs[0];
            let mut property = ParsedProperty::new(field.clone());
            if let Err(e) = attr.parse_nested_meta(|nested_meta| {
                let ident = nested_meta.path.require_ident()?;
                let ident = ident.to_string();

                match ident.as_str() {
                    "name" => {
                        if property.name.is_empty() {
                            let name: LitStr = nested_meta.value()?.parse()?;
                            let name = name.value();
                            if name.is_empty() {
                                self.errored = true;
                                return Err(nested_meta.error("field name cannot be empty"));
                            }
                            property.name = name;
                        } else {
                            self.errored = true;
                            return Err(nested_meta.error("already specified"));
                        }
                    },
                    "readonly" => {
                        if let None = property.readonly {
                            if let Ok(ts) = nested_meta.value() {
                                let b: LitBool = ts.parse()?;
                                property.readonly = Some(b.value());
                            } else {
                                property.readonly = Some(true);
                            }
                        } else {
                            self.errored = true;
                            return Err(nested_meta.error("already specified"));
                        }
                    },
                    "get" => {
                        if let None = property.get {
                            let name: LitStr = nested_meta.value()?.parse()?;
                            let name = name.value();
                            if name.is_empty() {
                                self.errored = true;
                                return Err(nested_meta.error("getter name cannot be empty"))?;
                            }
                            property.get = Some(name);
                        } else {
                            self.errored = true;
                            return Err(nested_meta.error("already specified"));
                        }
                    },
                    "set" => {
                        if let None = property.set {
                            let name: LitStr = nested_meta.value()?.parse()?;
                            let name = name.value();
                            if name.is_empty() {
                                self.errored = true;
                                return Err(nested_meta.error("setter name cannot be empty"));
                            }
                            property.set = Some(name);
                        } else {
                            self.errored = true;
                            return Err(nested_meta.error("already specified"));
                        }
                    },
                    "security_context" => {
                        if let None = property.security_context {
                            let value: Ident = nested_meta.value()?.parse()?;

                            let ident = value.to_string();

                            match ident.as_str() {
                                "None" => {
                                    property.security_context = Some(SecurityContext::None);
                                },
                                "PluginSecurity" => {
                                    property.security_context = Some(SecurityContext::PluginSecurity);
                                },
                                "LocalUserSecurity" => {
                                    property.security_context = Some(SecurityContext::LocalUserSecurity);
                                },
                                "RobloxScriptSecurity" => {
                                    property.security_context = Some(SecurityContext::RobloxScriptSecurity);
                                },
                                _ => {
                                    self.errored = true;
                                    return Err(nested_meta.error("unknown secuirty context"));
                                }
                            }
                        } else {
                            self.errored = true;
                            return Err(nested_meta.error("already specified"));
                        }
                    },
                    _ => {
                        self.errored = true;
                        return Err(nested_meta.error("unknown attribute"));
                    }
                }
                Ok(())
            }) {
                self.errors.extend(e.into_compile_error());
                self.errored = true;
                return
            };

            self.prop.push(property);
        } else if attrs.len() > 1 {
            for a in attrs {
                let ts = Error::new(a.span(), "too much instance specifiers").into_compile_error();
                self.errors.extend(ts);
                self.errored = true;
            }
        }
    }
}

struct ParsedInstanceData {

}

fn parse_instance_attr(attr: &syn::Attribute) -> syn::Result<ParsedInstanceData> {
    todo!()
}

#[proc_macro_derive(Instance, attributes(instance, property, method))]
pub fn instance_derive(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let origin: syn::DeriveInput = syn::parse_macro_input!(ts);

    // todo: capture `#[instance]` from origin.attrs
    let attrs: Vec<&Attribute> = origin.attrs.iter().filter(|attr| attr.meta.path().is_ident("instance")).collect();
    if attrs.len() > 1 {
        let mut errors = proc_macro2::TokenStream::new();
        for a in attrs {
            let ts = Error::new(a.span(), "too much instance specifiers").into_compile_error();
            errors.extend(ts);
        }
        return errors.into()
    } else if attrs.is_empty() {
        return syn::Error::new(Span::call_site(), "`#![instance]` attr must be specified").into_compile_error().into()
    }

    let pid = match parse_instance_attr(attrs[0]) {
        Ok(pid) => pid,
        Err(e) => return e.to_compile_error().into(),
    };

    // visitor?
    let properties = if let Data::Struct(ds) = origin.data {
        let mut vis = InstanceDeriveVisitor {
            errors: TokenStream::new(),
            prop: vec![],
            errored: false
        };

        vis.visit_data_struct(&ds);

        if vis.errored {
            return vis.errors.into()
        }

        vis.prop
    } else {
        return syn::Error::new(Span::call_site(), "instance must be a struct").into_compile_error().into()
    };
    
    todo!()
}

#[proc_macro_attribute]
pub fn methods(_item: proc_macro::TokenStream, ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let origin: syn::ItemImpl = syn::parse_macro_input!(ts);

    let items = origin.items;

    let q: syn::ItemImpl = parse_quote! {
        impl dyn IDataModel {
            #(#items)*
        }
    };

    q.into_token_stream().into()
}