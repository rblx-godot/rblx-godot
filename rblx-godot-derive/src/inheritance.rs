use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn create_inheritance_map(struct_name: &Ident, classes: &Vec<Ident>) -> TokenStream {
    let v: Vec<TokenStream> = classes
        .iter()
        .map(|class| {
            let iclass = Ident::new(format!("I{}", class.to_string()).as_str(), class.span());
            quote! {
                .insert_type::<#struct_name, dyn crate::instance::#iclass>(|x| x, |x| x)
            }
        })
        .collect();
    quote! {
        impl crate::core::InheritanceBase for #struct_name {
            fn inheritance_table(&self) -> crate::core::InheritanceTable {
                crate::core::InheritanceTableBuilder::new()
                    #(#v)*
                    .insert_type::<#struct_name, #struct_name>(|x| x, |x| x)
                    .output()
            }
        }
    }
}
