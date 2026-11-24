use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, Generics, Ident};

use super::helper::state_element_helper;

pub fn derive_state_element_struct(
    data_struct: DataStruct,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (initializers, initializers_mut, is_unnamed, _window_title, _window_class) =
        state_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // TODO: Instead get this from the proc macro.
    let impl_for = match std::env::var("CARGO_PKG_NAME").unwrap() == "ro-next" {
        true => Some(quote!(crate::state::ClientState)),
        false => None,
    };

    if let Some(impl_for) = impl_for {
        if initializers.len() == 1 && is_unnamed {
            return quote! {
                impl #impl_generics ro_next_interface::element::StateElement<#impl_for> for #name #type_generics #where_clause {
                    type LayoutInfoMut = impl std::any::Any;
                    type ReturnMut<P>
                        = impl ro_next_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfoMut>
                    where
                        P: rust_state::Path<#impl_for, Self>;
                    type LayoutInfo = impl std::any::Any;
                    type Return<P>
                        = impl ro_next_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfo>
                    where
                        P: rust_state::Path<#impl_for, Self>;

                    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                        where P: rust_state::Path<#impl_for, Self>
                    {
                        ro_next_interface::element::StateElement::to_element(self_path._0(), name)
                    }

                    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                        where P: rust_state::Path<#impl_for, Self>
                    {
                        ro_next_interface::element::StateElement::to_element_mut(self_path._0(), name)
                    }
                }
            }
            .into();
        }

        return quote! {
            impl #impl_generics ro_next_interface::element::StateElement<#impl_for> for #name #type_generics #where_clause {
                type LayoutInfoMut = impl std::any::Any;
                type ReturnMut<P>
                    = impl ro_next_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfoMut>
                where
                    P: rust_state::Path<#impl_for, Self>;
                type LayoutInfo = impl std::any::Any;
                type Return<P>
                    = impl ro_next_interface::element::Element<#impl_for, LayoutInfo = Self::LayoutInfo>
                where
                    P: rust_state::Path<#impl_for, Self>;

                fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                    where P: rust_state::Path<#impl_for, Self>
                {
                    use ro_next_interface::prelude::*;

                    collapsible! {
                        text: name,
                        children: (#(#initializers,)*),
                    }
                }

                fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                    where P: rust_state::Path<#impl_for, Self>
                {
                    use ro_next_interface::prelude::*;

                    collapsible! {
                        text: name,
                        children: (#(#initializers_mut,)*),
                    }
                }
            }
        }
        .into();
    }

    if initializers.len() == 1 && is_unnamed {
        return quote! {
            impl<App: ro_next_interface::application::Application> #impl_generics ro_next_interface::element::StateElement<App> for #name #type_generics #where_clause {
                type LayoutInfoMut = impl std::any::Any;
                type ReturnMut<P>
                    = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
                where
                    P: rust_state::Path<App, Self>;
                type LayoutInfo = impl std::any::Any;
                type Return<P>
                    = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
                where
                    P: rust_state::Path<App, Self>;

                fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                    where P: rust_state::Path<App, Self>
                {
                    ro_next_interface::element::StateElement::to_element(self_path._0(), name)
                }

                fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                    where P: rust_state::Path<App, Self>
                {
                    ro_next_interface::element::StateElement::to_element_mut(self_path._0(), name)
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: ro_next_interface::application::Application> #impl_generics ro_next_interface::element::StateElement<App> for #name #type_generics #where_clause {
            type LayoutInfoMut = impl std::any::Any;
            type ReturnMut<P>
                = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
            where
                P: rust_state::Path<App, Self>;
            type LayoutInfo = impl std::any::Any;
            type Return<P>
                = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
            where
                P: rust_state::Path<App, Self>;

            fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                where P: rust_state::Path<App, Self>
            {
                use ro_next_interface::prelude::*;

                collapsible! {
                    text: name,
                    children: (#(#initializers,)*),
                }
            }

            fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                where P: rust_state::Path<App, Self>
            {
                use ro_next_interface::prelude::*;

                collapsible! {
                    text: name,
                    children: (#(#initializers_mut,)*),
                }
            }
        }
    }
    .into()
}

pub fn derive_state_element_enum(_data_enum: DataEnum, generics: Generics, name: Ident) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // let name_string = name.to_string();

    // let variant_names = data_enum.variants.iter().map(|variant|
    // variant.ident.clone()).collect::<Vec<_>>(); let variant_matchers =
    // data_enum.variants     .iter()
    //     .map(|variant| match variant.fields {
    //         syn::Fields::Named(..) => quote!( { .. } ),
    //         syn::Fields::Unnamed(..) => quote!( (..) ),
    //         syn::Fields::Unit => quote!(),
    //     })
    //     .collect::<Vec<_>>();
    // let element_names = variant_names
    //     .iter()
    //     .map(|ident| syn::Ident::new(&format!("{ident}_element"),
    // Span::mixed_site()))     .collect::<Vec<_>>();
    // let store_indices = (0..data_enum.variants.len())
    //     .map(|index| syn::LitInt::new(&index.to_string(), Span::mixed_site()))
    //     .collect::<Vec<_>>();
    // let layout_info_generics = variant_names
    //     .iter()
    //     .map(|ident| syn::Ident::new(&format!("__{ident}"), Span::mixed_site()))
    //     .collect::<Vec<_>>();
    //
    // let elements = data_enum
    //     .variants
    //     .iter()
    //     .map(|variant| {
    //         let variant_string = variant.ident.to_string();
    //
    //         quote! {
    //             split! {
    //                 children: (
    //                     text! {
    //                         text: #name_string,
    //                     },
    //                     field! {
    //                         text: #variant_string,
    //                     },
    //                 ),
    //             }
    //         }
    //     });
    //
    // let elements_mut = data_enum
    //     .variants
    //     .iter()
    //     .map(|variant| {
    //         let variant_string = variant.ident.to_string();
    //
    //         quote! {
    //             split! {
    //                 children: (
    //                     text! {
    //                         text: #name_string,
    //                     },
    //                     field! {
    //                         text: #variant_string,
    //                     },
    //                 ),
    //             }
    //         }
    //     });

    // TODO: Handle attributes like hidden_element

    // if std::env::var("CARGO_PKG_NAME").unwrap() == "ro-next" {
    //     return quote! {
    //         impl #impl_generics
    // ro_next_interface::element::StateElement<crate::interface::application::InterfaceSettings>
    // for #name #type_generics #where_clause {             fn
    // to_element(self_path: impl Path<App, Self>, name: String) -> impl
    // ro_next_interface::element::Element<crate::interface::application::InterfaceSettings>
    // {                 match self {
    //                     #( Self::#variants =>
    // ro_next_interface::element::StateElement::to_element(&#variant_strings,
    // display), )*                 }
    //             }
    //         }
    //     }
    //     .into();
    // }

    quote! {
        impl<App: ro_next_interface::application::Application> #impl_generics ro_next_interface::element::StateElement<App> for #name #type_generics #where_clause {
            type LayoutInfoMut = impl std::any::Any;
            type ReturnMut<P>
                = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfoMut>
            where
                P: rust_state::Path<App, Self>;
            type LayoutInfo = impl std::any::Any;
            type Return<P>
                = impl ro_next_interface::element::Element<App, LayoutInfo = Self::LayoutInfo>
            where
                P: rust_state::Path<App, Self>;

            fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
                where P: rust_state::Path<App, Self>
            {
                use ro_next_interface::prelude::*;

                // #[derive(Clone, Copy, PartialEq)]
                // enum InnerLayoutInfo<#(#layout_info_generics),*> {
                //     #(#variant_names(#layout_info_generics)),*
                // }
                //
                // struct Inner<P, #(#layout_info_generics),*> {
                //     path: P,
                //     #(
                //         #[allow(non_snake_case)]
                //         #element_names: #layout_info_generics,
                //     )*
                // }
                //
                // impl<App, P, #(#layout_info_generics),*> ro_next_interface::element::Element<App> for Inner<P, #(#layout_info_generics),*>
                // where
                //     App: ro_next_interface::application::Application,
                //     P: rust_state::Path<App, #name>,
                //     #(#layout_info_generics: ro_next_interface::element::Element<App>),*
                // {
                //     type LayoutInfo = InnerLayoutInfo< #(#layout_info_generics::LayoutInfo),* >;
                //
                //     fn create_layout_info(
                //         &mut self,
                //         state: &rust_state::State<App>,
                //         mut store: ro_next_interface::element::store::ElementStoreMut,
                //         resolvers: &mut ro_next_interface::layout::Resolver<App>,
                //     ) -> Self::LayoutInfo {
                //         match state.get(&self.path) {
                //             #(
                //                 #name::#variant_names #variant_matchers => {
                //                     InnerLayoutInfo::#variant_names(self.#element_names.create_layout_info(state, store.child_store(#store_indices), resolvers))
                //                 },
                //             )*
                //         }
                //     }
                //
                //     fn lay_out<'a>(
                //         &'a self,
                //         state: &'a rust_state::State<App>,
                //         store: ro_next_interface::element::store::ElementStore<'a>,
                //         layout_info: &'a Self::LayoutInfo,
                //         layout: &mut ro_next_interface::layout::Layout<'a, App>,
                //     ) {
                //         match layout_info {
                //             #(
                //                 InnerLayoutInfo::#variant_names(layout_info) => self.#element_names.lay_out(state, store.child_store(#store_indices), layout_info, layout),
                //             )*
                //         }
                //     }
                // }
                //
                // Inner {
                //     path: self_path,
                //     #(#element_names: #elements),*
                // }

                button! {
                    text: name,
                    event: |state: &rust_state::State<App>, _: &mut ro_next_interface::event::EventQueue<App>| {
                        println!("Just a dummy for now");
                    },
                }
            }

            fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
                where P: rust_state::Path<App, Self>
            {
                use ro_next_interface::prelude::*;

                // #[derive(Clone, Copy, PartialEq)]
                // enum InnerLayoutInfo<#(#layout_info_generics),*> {
                //     #(#variant_names(#layout_info_generics)),*
                // }
                //
                // struct Inner<P, #(#layout_info_generics),*> {
                //     path: P,
                //     #(
                //         #[allow(non_snake_case)]
                //         #element_names: #layout_info_generics,
                //     )*
                // }
                //
                // impl<App, P, #(#layout_info_generics),*> ro_next_interface::element::Element<App> for Inner<P, #(#layout_info_generics),*>
                // where
                //     App: ro_next_interface::application::Application,
                //     P: rust_state::Path<App, #name>,
                //     #(#layout_info_generics: ro_next_interface::element::Element<App>),*
                // {
                //     type LayoutInfo = InnerLayoutInfo< #(#layout_info_generics::LayoutInfo),* >;
                //
                //     fn create_layout_info(
                //         &mut self,
                //         state: &rust_state::State<App>,
                //         mut store: ro_next_interface::element::store::ElementStoreMut,
                //         resolvers: &mut ro_next_interface::layout::Resolver<App>,
                //     ) -> Self::LayoutInfo {
                //         match state.get(&self.path) {
                //             #(
                //                 #name::#variant_names #variant_matchers => {
                //                     InnerLayoutInfo::#variant_names(self.#element_names.create_layout_info(state, store.child_store(#store_indices), resolvers))
                //                 },
                //             )*
                //         }
                //     }
                //
                //     fn lay_out<'a>(
                //         &'a self,
                //         state: &'a rust_state::State<App>,
                //         store: ro_next_interface::element::store::ElementStore<'a>,
                //         layout_info: &'a Self::LayoutInfo,
                //         layout: &mut ro_next_interface::layout::Layout<'a, App>,
                //     ) {
                //         match layout_info {
                //             #(
                //                 InnerLayoutInfo::#variant_names(layout_info) => self.#element_names.lay_out(state, store.child_store(#store_indices), layout_info, layout),
                //             )*
                //         }
                //     }
                // }
                //
                // Inner {
                //     path: self_path,
                //     #(#element_names: #elements_mut),*
                // }

                        // match self {
        //     #( Self::#variants =>
        // ro_next_interface::element::StateElement::to_element(&#variant_strings,
        // display), )* }

                button! {
                    text: name,
                    event: |state: &rust_state::State<App>, _: &mut ro_next_interface::event::EventQueue<App>| {
                        println!("Just a dummy for now");
                    },
                }

            }
        }
    }
    .into()
}
