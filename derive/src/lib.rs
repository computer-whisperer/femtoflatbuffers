extern crate proc_macro;

use std::env::var;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, parse_macro_input, parse_quote};
use syn::token::Token;

fn add_trait_bounds(mut generics: syn::Generics) -> syn::Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(femtoflatbuffers::ComponentEncode));
            type_param
                .bounds
                .push(parse_quote!(femtoflatbuffers::ComponentDecode));
        }
    }
    generics
}

#[proc_macro_derive(Table)]
pub fn flatbuffers_table_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let encode = do_encode_table(&input.data);
    let root_offset_ident = format_ident!("root_offset");
    let decode = do_decode_table(name.clone(), &input.data, root_offset_ident.clone());

    let expanded = quote! {
        impl #impl_generics femtoflatbuffers::table::Table for #name #ty_generics #where_clause {
            fn encode(&self, encoder: &mut femtoflatbuffers::Encoder) -> Result<(), femtoflatbuffers::EncodeError> {
                encoder.encode_u32(4)?;
                {
                  #encode
                }?;
                Ok(())
            }

            fn decode(decoder: &femtoflatbuffers::Decoder) -> Result<Self, femtoflatbuffers::DecodeError> {
                let root_offset = decoder.decode_u32(0)?;
                #decode
            }
        }
        impl #impl_generics femtoflatbuffers::ComponentEncode for #name #ty_generics #where_clause {
            type WorkingValue = (u32, u32);
            fn value_encode(&self, encoder: &mut femtoflatbuffers::Encoder, table_start: u32) -> Result<Self::WorkingValue, femtoflatbuffers::EncodeError> {
                let value_offset = encoder.encode_i32(0)?;
                Ok((table_start, value_offset))
            }
            fn vtable_encode(&self, encoder: &mut femtoflatbuffers::Encoder, _vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), femtoflatbuffers::EncodeError> {
                encoder.encode_u16((working_value.1 - working_value.0) as u16)?;
                Ok(())
            }
            fn post_encode(&self, encoder: &mut femtoflatbuffers::Encoder, working_value: &Self::WorkingValue) -> Result<(), femtoflatbuffers::EncodeError> {
                match {
                    #encode
                } {
                    Ok(global_table_offset) => {
                        let global_field_offset = working_value.1;
                        encoder.encode_i32_at(global_field_offset, (global_table_offset - global_field_offset) as i32)?;
                        Ok(())
                    },
                    Err(err) => Err(err)
                }
            }
        }
        impl #impl_generics femtoflatbuffers::ComponentDecode for #name #ty_generics #where_clause {
            type WorkingValue = (u32, u16);
            type VectorWorkingValue = Self::WorkingValue;
            fn vtable_decode(decoder: &femtoflatbuffers::Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), femtoflatbuffers::DecodeError> {
                let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
                Ok(((table_start, vtable_entry_value), vtable_entry+2))
            }
            fn value_decode(decoder: &femtoflatbuffers::Decoder, working_value: &Self::WorkingValue) -> Result<Self, femtoflatbuffers::DecodeError> {
                let #root_offset_ident = ((working_value.0 + working_value.1 as u32) as i32 + decoder.decode_i32(working_value.0 + working_value.1 as u32)?) as u32;
                #decode
            }
            fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), femtoflatbuffers::DecodeError> {
                let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
                Ok(((table_start, vtable_entry_value), vtable_entry+2))
            }
            fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, femtoflatbuffers::DecodeError> {
                let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32) as u32;
                Ok(decoder.decode_u32(vector_offset)? as usize)
            }
            fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, femtoflatbuffers::DecodeError>
            where
                Self: Sized
            {
                let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32) as u32;
                let vector_entry_offset = (vector_offset+4) + (idx*4) as u32;
                let #root_offset_ident = (vector_entry_offset as i32 + decoder.decode_i32(vector_entry_offset)?) as u32;
                #decode
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}


fn inner_do_table_encode(
    table_start_ident: Ident,
    vtable_start_ident: Ident,
    fields_encode: &[TokenStream],
    offsets_encode: &[TokenStream],
    post_encode: &[TokenStream]
) -> TokenStream {
    quote! {
        // Write the table itself
        let #table_start_ident = encoder.encode_i32(0)?;
        #(#fields_encode)*
        let table_end = encoder.used_bytes();
        // Write the vtable
        let #vtable_start_ident = encoder.encode_u16(0)?;
        encoder.encode_i32_at(#table_start_ident, -((#vtable_start_ident - #table_start_ident) as i32))?; // Set vtable offset
        encoder.encode_u16((table_end - #table_start_ident) as u16)?; // Set table size
        // Set field offsets
        #(#offsets_encode)*
        // Write the start table offset
        encoder.encode_u16_at(#vtable_start_ident, (encoder.used_bytes() - #vtable_start_ident) as u16)?;
        #(#post_encode)*
        Ok(#table_start_ident)
    }
}

fn do_encode_table(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        match data.fields {
            syn::Fields::Named(ref fields) => {
                let mut fields_encode = Vec::new();
                let mut offsets_encode = Vec::new();
                let mut post_encodes = Vec::new();
                let table_start_ident = format_ident!("start");
                let vtable_start_ident = format_ident!("vtable_start");
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap();
                    let working_value_name = format_ident!("{}_working_value", field_name);
                    fields_encode.push(quote! {
                        let #working_value_name = femtoflatbuffers::ComponentEncode::value_encode(&self.#field_name, encoder, #table_start_ident)?;
                    });
                    offsets_encode.push(quote! {
                        femtoflatbuffers::ComponentEncode::vtable_encode(&self.#field_name, encoder, #vtable_start_ident, &#working_value_name)?;
                    });
                    post_encodes.push(quote! {
                        femtoflatbuffers::ComponentEncode::post_encode(&self.#field_name, encoder, &#working_value_name)?;
                    });
                }
                inner_do_table_encode(table_start_ident, vtable_start_ident, &fields_encode, &offsets_encode, &post_encodes)
            }
            _ => panic!("Only named fields are supported"),
        }
    } else {
        panic!("Only structs are supported");
    }
}

fn do_decode_table(type_name: Ident, data: &Data, table_start_ident: Ident) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        match data.fields {
            syn::Fields::Named(ref fields) => {
                let mut offset_calcs = Vec::new();
                let mut struct_populations = Vec::new();
                let offset_ident = format_ident!("offset");
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_type_name = &field.ty;
                    let working_value_ident = format_ident!("{}_working_value", field_name);
                    offset_calcs.push(quote! {
                        let (#working_value_ident, #offset_ident) = <#field_type_name as femtoflatbuffers::ComponentDecode>::vtable_decode(&decoder, #table_start_ident, #offset_ident)?;
                    });
                    struct_populations.push(quote! {
                        #field_name: <#field_type_name as femtoflatbuffers::ComponentDecode>::value_decode(&decoder, &#working_value_ident)?
                    });
                }
                quote! {
                    let vtable_offset = ((#table_start_ident as i32) - decoder.decode_i32(#table_start_ident)?) as u32;
                    let vtable_size = decoder.decode_u16(vtable_offset)?;
                    let table_size = decoder.decode_u16(vtable_offset + 2)?;
                    let #offset_ident = vtable_offset + 4;
                    #(#offset_calcs)*
                    let res = #type_name {
                        #(#struct_populations,)*
                    };
                    Ok(res)
                }
            }
            _ => panic!("Only named fields are supported"),
        }
    } else {
        panic!("Only structs are supported");
    }
}

#[proc_macro_derive(Union)]
pub fn flatbuffers_union_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let expanded = if let Data::Enum(ref data) = input.data {
        let mut variant_id = 0u8;
        let encode_working_value_enum_ident = format_ident!("EncodeWorkingValue{}", name);
        let decode_working_value_enum_ident = format_ident!("DecodeWorkingValue{}", name);
        let mut encode_working_value_enum_arms = vec![];
        let mut decode_working_value_enum_arms = vec![];
        let mut value_encode_match_cases = vec![];
        let mut vtable_encode_match_cases = vec![];
        let mut post_encode_match_cases = vec![];
        let mut vtable_decode_match_cases = vec![];
        let mut decode_match_cases = vec![];
        for variant in &data.variants {
            let variant_ident = variant.ident.clone();
            if variant_id == 0 {
                value_encode_match_cases.push(quote!{
                    #name::#variant_ident => {
                        Err(femtoflatbuffers::EncodeError::InvalidStructure)
                    }
                });
            }
            else {
                let variant_field = variant.fields.iter().next().unwrap();
                let variant_type = &variant_field.ty;
                let enum_arm_ident = format_ident!("{}_arm", variant_ident);
                encode_working_value_enum_arms.push(quote!{
                    #enum_arm_ident(<#variant_type as femtoflatbuffers::ComponentEncode>::WorkingValue)
                });
                decode_working_value_enum_arms.push(quote!{
                    #enum_arm_ident(<#variant_type as femtoflatbuffers::ComponentDecode>::WorkingValue)
                });
                value_encode_match_cases.push(quote!{
                    #name::#variant_ident(field, ..) => {
                        let res = encoder.encode_u8(#variant_id)?;
                        let value_res = femtoflatbuffers::ComponentEncode::value_encode(field, encoder, table_start)?;
                        Ok(((table_start, res), #encode_working_value_enum_ident::#enum_arm_ident(value_res)))
                    }
               });
                vtable_encode_match_cases.push(quote!{
                    (#name::#variant_ident(field, ..), ((table_start, which_offset), #encode_working_value_enum_ident::#enum_arm_ident(working_value))) => {
                        encoder.encode_u16((which_offset - table_start) as u16)?;
                        femtoflatbuffers::ComponentEncode::vtable_encode(field, encoder, vtable_start, working_value)
                    }
               });
                post_encode_match_cases.push(quote!{
                    (#name::#variant_ident(field), #encode_working_value_enum_ident::#enum_arm_ident(working_value)) => {
                        femtoflatbuffers::ComponentEncode::post_encode(field, encoder, working_value)?;
                    }
                });
                vtable_decode_match_cases.push(quote!{
                    #variant_id => {
                        let (working_value, next_offset) = <#variant_type as femtoflatbuffers::ComponentDecode>::vtable_decode(decoder, table_start, vtable_entry+2)?;
                        Ok((#decode_working_value_enum_ident::#enum_arm_ident(working_value), next_offset))
                    }
                });
                decode_match_cases.push(quote!{
                    #decode_working_value_enum_ident::#enum_arm_ident(inner_working_value) => {
                        Ok(#name::#variant_ident(<#variant_type as femtoflatbuffers::ComponentDecode>::value_decode(decoder, inner_working_value)?))
                    }
                });
            }
            variant_id += 1;
        }
        let expanded = quote! {
            enum #encode_working_value_enum_ident {
                #(#encode_working_value_enum_arms,)*
            }
            enum #decode_working_value_enum_ident {
                #(#decode_working_value_enum_arms,)*
            }
            impl #impl_generics femtoflatbuffers::ComponentEncode for #name #ty_generics #where_clause {
                type WorkingValue = ((u32, u32), #encode_working_value_enum_ident);
                fn value_encode(&self, encoder: &mut femtoflatbuffers::Encoder, table_start: u32) -> Result<Self::WorkingValue, femtoflatbuffers::EncodeError> {
                    match self {
                        #(#value_encode_match_cases)*
                        _ => {
                            Err(femtoflatbuffers::EncodeError::InvalidStructure)
                        }
                    }
                }
                fn vtable_encode(&self, encoder: &mut femtoflatbuffers::Encoder, vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), femtoflatbuffers::EncodeError> {
                    match (self, working_value) {
                        #(#vtable_encode_match_cases)*
                        _ => {
                            encoder.encode_u16(0)?;
                            encoder.encode_u16(0)?;
                            Ok(())
                        }
                    }
                }
                fn post_encode(&self, encoder: &mut femtoflatbuffers::Encoder, working_value: &Self::WorkingValue) -> Result<(), femtoflatbuffers::EncodeError> {
                    match (self, &working_value.1) {
                        #(#post_encode_match_cases)*
                        _ => {}
                    }
                    Ok(())
                }
            }
            impl #impl_generics femtoflatbuffers::ComponentDecode for #name #ty_generics #where_clause {
                type WorkingValue = #decode_working_value_enum_ident;
                type VectorWorkingValue = ();
                fn vtable_decode(decoder: &femtoflatbuffers::Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), femtoflatbuffers::DecodeError> {
                    let which_offset = decoder.decode_u16(vtable_entry)?;
                    if which_offset != 0 {
                        let which_value = decoder.decode_u8((which_offset as u32) + table_start)?;
                        let (inner_working_value, next_offset) = match which_value {
                            #(#vtable_decode_match_cases)*
                            _ => {
                                Err(femtoflatbuffers::DecodeError::InvalidData)
                            }
                        }?;
                        Ok((inner_working_value, next_offset))
                    }
                    else {
                        Err(femtoflatbuffers::DecodeError::InvalidData)
                    }
                }
                fn value_decode(decoder: &femtoflatbuffers::Decoder, working_value: &Self::WorkingValue) -> Result<Self, femtoflatbuffers::DecodeError> {
                    match working_value {
                        #(#decode_match_cases,)*
                        _ => {
                            Err(femtoflatbuffers::DecodeError::InvalidData)
                        }
                    }
                }
                fn vector_vtable_decode(decoder: &femtoflatbuffers::Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), femtoflatbuffers::DecodeError> {
                    Err(femtoflatbuffers::DecodeError::UnsupportedFeature)
                }
                fn vector_len_decode(decoder: &femtoflatbuffers::Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, femtoflatbuffers::DecodeError> {
                    Err(femtoflatbuffers::DecodeError::UnsupportedFeature)
                }
                fn vector_value_decode(decoder: &femtoflatbuffers::Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, femtoflatbuffers::DecodeError> {
                    Err(femtoflatbuffers::DecodeError::UnsupportedFeature)
                }
            }
        };
        expanded
    }
    else {
        panic!("Only enum are supported");
    };
    proc_macro::TokenStream::from(expanded)
}


#[cfg(test)]
mod tests {}
