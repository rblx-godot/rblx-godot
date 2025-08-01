use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Ident, Result, Token,
};

#[derive(Debug, Clone)]
pub(crate) enum AttrArgValue {
    Expr(syn::Expr),
    Func(syn::Signature),
}
#[derive(Debug, Clone)]
pub(crate) enum AttrArgValueRecursive {
    Expr(syn::Expr),
    Func(syn::Signature),
    Recursive(Ident, syn::token::Paren, AttrArgumentsRecursive),
}
#[derive(Debug, Clone)]
pub(crate) struct AttrNamedArg {
    pub name: Ident,
    pub assign_token: Token![=],
    pub value: AttrArgValue,
}
#[derive(Debug, Clone)]
pub(crate) struct AttrNamedArgRecursive {
    pub name: Ident,
    pub assign_token: Token![=],
    pub value: AttrArgValueRecursive,
}
#[derive(Debug, Clone)]
pub(crate) enum AttrArg {
    Named(AttrNamedArg),
    Unnamed(AttrArgValue),
}
#[derive(Debug, Clone)]
pub(crate) enum AttrArgRecursive {
    Named(AttrNamedArgRecursive),
    Unnamed(AttrArgValueRecursive),
}

#[derive(Debug, Clone)]
pub(crate) struct AttrArguments {
    pub args: Punctuated<AttrArg, Token![,]>,
}
#[derive(Debug, Clone)]
pub(crate) struct AttrArgumentsRecursive {
    pub args: Punctuated<AttrArgRecursive, Token![,]>,
}

impl Parse for AttrArgValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::Token![fn]) {
            println!("parse function signature");
            Ok(AttrArgValue::Func(input.parse()?))
        } else {
            Ok(AttrArgValue::Expr(input.parse()?))
        }
    }
}
impl Parse for AttrArgValueRecursive {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::Token![fn]) {
            Ok(AttrArgValueRecursive::Func(input.parse()?))
        } else if input.peek(syn::Ident) && input.peek2(syn::token::Paren) {
            let ident = input.parse()?;
            let content;
            let paren = syn::parenthesized!(content in input);
            let args = content.parse()?;
            Ok(AttrArgValueRecursive::Recursive(ident, paren, args))
        } else {
            Ok(AttrArgValueRecursive::Expr(input.parse()?))
        }
    }
}

impl Parse for AttrNamedArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let assign_token = input.parse()?;
        let value = input.parse()?;
        Ok(AttrNamedArg {
            name,
            assign_token,
            value,
        })
    }
}

impl Parse for AttrArg {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek2(Token![=]) {
            Ok(AttrArg::Named(input.parse()?))
        } else {
            Ok(AttrArg::Unnamed(input.parse()?))
        }
    }
}

impl Parse for AttrNamedArgRecursive {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let assign_token = input.parse()?;
        let value = input.parse()?;
        Ok(AttrNamedArgRecursive {
            name,
            assign_token,
            value,
        })
    }
}

impl Parse for AttrArgRecursive {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek2(Token![=]) {
            Ok(AttrArgRecursive::Named(input.parse()?))
        } else {
            Ok(AttrArgRecursive::Unnamed(input.parse()?))
        }
    }
}

impl Parse for AttrArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(AttrArguments {
                args: Punctuated::new(),
            });
        }
        Ok(AttrArguments {
            args: input.parse_terminated(AttrArg::parse, Token![,])?,
        })
    }
}

impl Parse for AttrArgumentsRecursive {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(AttrArgumentsRecursive {
                args: Punctuated::new(),
            });
        }
        Ok(AttrArgumentsRecursive {
            args: input.parse_terminated(AttrArgRecursive::parse, Token![,])?,
        })
    }
}

impl ToTokens for AttrArgValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AttrArgValue::Expr(expr) => expr.to_tokens(tokens),
            AttrArgValue::Func(sig) => sig.to_tokens(tokens),
        }
    }
}
impl ToTokens for AttrArgValueRecursive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AttrArgValueRecursive::Expr(expr) => expr.to_tokens(tokens),
            AttrArgValueRecursive::Func(sig) => sig.to_tokens(tokens),
            AttrArgValueRecursive::Recursive(ident, paren, args) => {
                ident.to_tokens(tokens);
                paren.surround(tokens, |tokens| args.to_tokens(tokens));
            }
        }
    }
}

impl TryInto<syn::Expr> for AttrArgValue {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Expr> {
        println!("try into expr");
        match self {
            AttrArgValue::Expr(expr) => Ok(expr),
            x => Err(syn::Error::new(x.span(), "expected expression")),
        }
    }
}

impl TryInto<syn::Signature> for AttrArgValue {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Signature> {
        match self {
            AttrArgValue::Func(sig) => Ok(sig),
            x => Err(syn::Error::new(x.span(), "expected function")),
        }
    }
}

impl TryInto<syn::Path> for AttrArgValue {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Path> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            Ok(p.path)
        } else {
            Err(syn::Error::new(span, "expected path"))
        }
    }
}

impl TryInto<syn::Ident> for AttrArgValue {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Ident> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            if p.path.segments.len() == 1 {
                Ok(p.path.segments[0].ident.clone())
            } else {
                Err(syn::Error::new(
                    p.path.span(),
                    "expected identifier, got path",
                ))
            }
        } else {
            Err(syn::Error::new(span, "expected identifier"))
        }
    }
}

impl TryInto<syn::Expr> for AttrArgValueRecursive {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Expr> {
        match self {
            AttrArgValueRecursive::Expr(expr) => Ok(expr),
            x => Err(syn::Error::new(x.span(), "expected expression")),
        }
    }
}

impl TryInto<syn::Signature> for AttrArgValueRecursive {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Signature> {
        match self {
            AttrArgValueRecursive::Func(sig) => Ok(sig),
            x => Err(syn::Error::new(x.span(), "expected function")),
        }
    }
}

impl TryInto<syn::Path> for AttrArgValueRecursive {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Path> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            Ok(p.path)
        } else {
            Err(syn::Error::new(span, "expected path"))
        }
    }
}

impl TryInto<syn::Ident> for AttrArgValueRecursive {
    type Error = syn::Error;

    fn try_into(self) -> Result<syn::Ident> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            if p.path.segments.len() == 1 {
                Ok(p.path.segments[0].ident.clone())
            } else {
                Err(syn::Error::new(
                    p.path.span(),
                    "expected identifier, got path",
                ))
            }
        } else {
            Err(syn::Error::new(span, "expected identifier"))
        }
    }
}

impl TryInto<String> for AttrArgValue {
    type Error = syn::Error;

    fn try_into(self) -> Result<String> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            if p.path.segments.len() == 1 {
                Ok(p.path.segments[0].ident.to_string())
            } else {
                Err(syn::Error::new(
                    p.path.span(),
                    "expected identifier, got path",
                ))
            }
        } else if let Self::Expr(syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit_str),
            ..
        })) = self
        {
            Ok(lit_str.value())
        } else {
            Err(syn::Error::new(span, "expected identifier"))
        }
    }
}
impl TryInto<String> for AttrArgValueRecursive {
    type Error = syn::Error;

    fn try_into(self) -> Result<String> {
        let span = self.span();
        if let Self::Expr(syn::Expr::Path(p)) = self {
            if p.path.segments.len() == 1 {
                Ok(p.path.segments[0].ident.to_string())
            } else {
                Err(syn::Error::new(
                    p.path.span(),
                    "expected identifier, got path",
                ))
            }
        } else if let Self::Expr(syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit_str),
            ..
        })) = self
        {
            Ok(lit_str.value())
        } else {
            Err(syn::Error::new(span, "expected identifier"))
        }
    }
}

impl AttrArguments {
    pub fn get_named_arg(&self, name: &str) -> Option<&AttrNamedArg> {
        self.args.iter().find_map(|arg| match arg {
            AttrArg::Named(named) => {
                if named.name == name {
                    Some(named)
                } else {
                    None
                }
            }
            _ => None,
        })
    }
    pub fn get_named_args(&self) -> impl Iterator<Item = &AttrNamedArg> {
        self.args.iter().filter_map(|arg| match arg {
            AttrArg::Named(named) => Some(named),
            _ => None,
        })
    }
    pub fn get_unnamed_arg(&self, idx: usize) -> Option<&AttrArgValue> {
        self.args
            .iter()
            .filter_map(|x| match x {
                AttrArg::Unnamed(v) => Some(v),
                _ => None,
            })
            .nth(idx)
    }
    pub fn get_unnamed_arg_count(&self) -> usize {
        self.args
            .iter()
            .filter_map(|x| match x {
                AttrArg::Unnamed(_) => Some(()),
                _ => None,
            })
            .count()
    }
    pub fn has_unnamed_arg(&self, name: &str) -> bool {
        self.args.iter().any(|arg| match arg {
            AttrArg::Unnamed(AttrArgValue::Expr(n)) => {
                if let syn::Expr::Path(p) = n {
                    p.path.is_ident(name)
                } else if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = n
                {
                    lit_str.value() == name
                } else {
                    false
                }
            }
            _ => false,
        })
    }
    pub fn get_arg(&self, idx: usize) -> Option<&AttrArg> {
        self.args.iter().nth(idx)
    }
}

impl ToTokens for AttrNamedArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.name.to_tokens(tokens);
        self.assign_token.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

impl ToTokens for AttrArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AttrArg::Named(named) => named.to_tokens(tokens),
            AttrArg::Unnamed(value) => value.to_tokens(tokens),
        }
    }
}

impl ToTokens for AttrArguments {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.args.to_tokens(tokens);
    }
}

impl AttrArgumentsRecursive {
    pub fn get_named_arg(&self, name: &str) -> Option<&AttrNamedArgRecursive> {
        self.args.iter().find_map(|arg| match arg {
            AttrArgRecursive::Named(named) => {
                if named.name == name {
                    Some(named)
                } else {
                    None
                }
            }
            _ => None,
        })
    }
    pub fn get_named_args(&self) -> impl Iterator<Item = &AttrNamedArgRecursive> {
        self.args.iter().filter_map(|arg| match arg {
            AttrArgRecursive::Named(named) => Some(named),
            _ => None,
        })
    }
    pub fn get_unnamed_arg(&self, idx: usize) -> Option<&AttrArgValueRecursive> {
        self.args
            .iter()
            .filter_map(|x| match x {
                AttrArgRecursive::Unnamed(v) => Some(v),
                _ => None,
            })
            .nth(idx)
    }
    pub fn get_unnamed_arg_count(&self) -> usize {
        self.args
            .iter()
            .filter_map(|x| match x {
                AttrArgRecursive::Unnamed(_) => Some(()),
                _ => None,
            })
            .count()
    }
}

impl ToTokens for AttrNamedArgRecursive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.name.to_tokens(tokens);
        self.assign_token.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

impl ToTokens for AttrArgRecursive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AttrArgRecursive::Named(named) => named.to_tokens(tokens),
            AttrArgRecursive::Unnamed(value) => value.to_tokens(tokens),
        }
    }
}

impl ToTokens for AttrArgumentsRecursive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.args.to_tokens(tokens);
    }
}
