use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident, LitInt};

use super::helper::byte_convertable_helper;
use crate::utils::{PacketSignature, get_unique_attribute};

pub fn derive_packet_struct(
    data_struct: DataStruct,
    generics: Generics,
    mut attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let packet_signature = get_unique_attribute(&mut attributes, "header")
        .map(|attribute| attribute.parse_args::<PacketSignature>())
        .expect("packet needs to specify a signature")
        .expect("failed to parse packet header");
    let is_ping = get_unique_attribute(&mut attributes, "ping").is_some();
    let is_variable_length = get_unique_attribute(&mut attributes, "variable_length").is_some();
    let expected_size = get_unique_attribute(&mut attributes, "packet_size")
        .map(|attribute| {
            attribute
                .parse_args::<LitInt>()
                .expect("`packet_size` expects one integer literal, the full wire size including the 2 byte header")
        })
        .map(|literal| {
            literal
                .base10_parse::<usize>()
                .expect("`packet_size` literal has to fit into usize")
        });
    // Opt-out for packets whose layout is deliberately not self-consistent.
    // Keep this explicit (and comment the struct) instead of silently skipping
    // the check for every packet.
    let allow_size_mismatch = get_unique_attribute(&mut attributes, "allow_packet_size_mismatch").is_some();

    if is_variable_length && expected_size.is_some() {
        panic!("a packet may not be `#[variable_length]` and specify `#[packet_size(..)]` at the same time");
    }

    if expected_size.is_some() && allow_size_mismatch {
        panic!("`#[allow_packet_size_mismatch]` has no effect together with `#[packet_size(..)]`");
    }

    // Fields that read "the rest of the packet" need the offset of the packet
    // payload in the generated parser (the helper references a local named
    // `base_offset`), so the bookkeeping has to be emitted for them even when
    // the size check itself is disabled.
    let has_length_remaining_field = data_struct.fields.iter().any(|field| {
        field.attrs.iter().any(|attribute| {
            let ident = attribute.path().segments[0].ident.to_string();

            ident == "length_remaining" || ident == "length_remaining_off_by_one"
        })
    });

    // The parser can only be checked against a size when one is known. Do not
    // emit the offset bookkeeping for packets that opted out to keep the
    // generated code (and the release hot path) free of unused work.
    let validation = if allow_size_mismatch {
        quote!()
    } else if is_variable_length {
        quote! {
            let __consumed_bytes = byte_reader.get_offset().saturating_sub(base_offset);
            ragnarok_packets::validation::check_variable_length(
                Self::HEADER,
                stringify!(#name),
                __packet_length as usize,
                __consumed_bytes,
            )?;
        }
    } else if let Some(expected_size) = expected_size {
        quote! {
            let __consumed_bytes = byte_reader.get_offset().saturating_sub(base_offset);
            ragnarok_packets::validation::check_fixed_length(
                Self::HEADER,
                stringify!(#name),
                #expected_size as usize,
                __consumed_bytes,
            )?;
        }
    } else {
        quote!()
    };

    let needs_base_offset = !validation.is_empty() || has_length_remaining_field;

    let insert_base_offset = needs_base_offset.then_some(quote! {
        let base_offset = byte_reader.get_offset();
    });

    let expected_size_const = match expected_size {
        Some(expected_size) => quote!(Some(#expected_size as usize)),
        None => quote!(None),
    };

    let signature = packet_signature.signature;
    let (new_implementation, from_bytes_implementations, implemented_fields, to_bytes_implementations, delimiter) =
        byte_convertable_helper(data_struct);

    let instanciate = match delimiter {
        proc_macro2::Delimiter::Brace => quote!(Self { #(#implemented_fields),* }),
        proc_macro2::Delimiter::Parenthesis => quote!(Self ( #(#implemented_fields),* )),
        _ => panic!(),
    };

    let insert_packet_length = is_variable_length.then_some(quote! {
        let __packet_length = ragnarok_bytes::ConversionResultExt::trace::<Self>(u16::from_bytes(byte_reader))?;
    });

    let final_to_bytes = match is_variable_length {
        _ if to_bytes_implementations.is_empty() => quote! {
            Ok(0)
        },
        true => {
            quote! {
                let start_position = byte_writer.len();
                let dummy_packet_length = 0u16;

                let written = byte_writer.write_counted(|writer| {
                    dummy_packet_length.to_bytes(writer)?;
                    #(#to_bytes_implementations)*
                    Ok(())
                })?;

                // We add 2 for the header bytes
                let packet_length = (written + 2) as u16;
                byte_writer.overwrite_at(start_position, packet_length.to_le_bytes())?;

                Ok(written)
            }
        }
        false => quote! {
            byte_writer.write_counted(|writer| {
                #(#to_bytes_implementations)*
                Ok(())
            })
        },
    };

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #new_implementation
        }

        impl #impl_generics ragnarok_packets::Packet for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;
            const IS_VARIABLE_LENGTH: bool = #is_variable_length;
            const EXPECTED_SIZE: Option<usize> = #expected_size_const;
            const HEADER: ragnarok_packets::PacketHeader = ragnarok_packets::PacketHeader(#signature);

            fn payload_from_bytes(byte_reader: &mut ragnarok_bytes::ByteReader) -> ragnarok_bytes::ConversionResult<Self> {
                #insert_base_offset
                #insert_packet_length
                #(#from_bytes_implementations)*
                let packet = #instanciate;
                #validation

                Ok(packet)
            }

            fn payload_to_bytes(&self, byte_writer: &mut ragnarok_bytes::ByteWriter) -> ragnarok_bytes::ConversionResult<usize> {
                #final_to_bytes
            }

            #[cfg(feature = "packet-to-state-element")]
            fn to_element<App: ro_next_interface::application::Application>(
                self_path: impl rust_state::Path<App, Self>,
                name: String,
            ) -> Box<dyn ro_next_interface::element::Element<App, LayoutInfo = ()>> {
                ro_next_interface::element::ErasedElement::new(<Self as ro_next_interface::element::StateElement<App>>::to_element(self_path, name))
            }
        }
    }
    .into()
}
