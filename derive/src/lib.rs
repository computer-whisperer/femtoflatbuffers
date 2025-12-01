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
            type TmpValue = u32;
            fn value_encode(&self, encoder: &mut femtoflatbuffers::Encoder) -> Result<Option<(u32, Self::TmpValue)>, femtoflatbuffers::EncodeError> {
                let value_offset = encoder.encode_i32(0)?;
                Ok(Some((value_offset, value_offset)))
            }
            fn post_encode(&self, encoder: &mut femtoflatbuffers::Encoder, tmp_value: Self::TmpValue) -> Result<(), femtoflatbuffers::EncodeError> {
                match {
                    #encode
                } {
                    Ok(global_table_offset) => {
                        let global_field_offset = tmp_value;
                        encoder.encode_i32_at(global_field_offset, (global_table_offset - global_field_offset) as i32)?;
                        Ok(())
                    },
                    Err(err) => Err(err)
                }
            }
        }
        impl #impl_generics femtoflatbuffers::ComponentDecode for #name #ty_generics #where_clause {
            fn value_decode(decoder: &femtoflatbuffers::Decoder, offset: Option<u32>) -> Result<Self, femtoflatbuffers::DecodeError> {
                if let Some(offset_offset) = offset {
                    let #root_offset_ident = (offset_offset as i32 + decoder.decode_i32(offset_offset)?) as u32;
                    #decode
                }
                else {
                    Err(femtoflatbuffers::DecodeError::InvalidData)
                }
            }
            fn table_value_size(decoder: &Decoder, table_value_global_offset: Option<u32>) -> Result<usize, femtoflatbuffers::DecodeError> {
                if let Some(_) = table_value_global_offset {
                    Ok(4)
                }
                else {
                    Ok(0)
                }
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}


fn inner_do_table_encode(
    start_offset_ident: Ident,
    fields_encode: &[TokenStream],
    offsets_encode: &[TokenStream],
    post_encode: &[TokenStream]
) -> TokenStream {
    quote! {
        // Write the table itself
        let #start_offset_ident = encoder.encode_i32(0)?;
        #(#fields_encode)*
        let table_end = encoder.used_bytes();
        // Write the vtable
        let start_vtable = encoder.encode_u16(0)?;
        encoder.encode_i32_at(#start_offset_ident, -((start_vtable - #start_offset_ident) as i32))?; // Set vtable offset
        encoder.encode_u16((table_end - #start_offset_ident) as u16)?; // Set table size
        // Set field offsets
        #(#offsets_encode)*
        // Write the start table offset
        encoder.encode_u16_at(start_vtable, (encoder.used_bytes() - start_vtable) as u16)?;
        #(#post_encode)*
        Ok(#start_offset_ident)
    }
}

fn do_encode_table(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        match data.fields {
            syn::Fields::Named(ref fields) => {
                let mut fields_encode = Vec::new();
                let mut offsets_encode = Vec::new();
                let mut post_encodes = Vec::new();
                let start_offset_ident = format_ident!("start");
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap();
                    let offset_name = format_ident!("{}_offset_blah", field_name);
                    fields_encode.push(quote! {
                        let #offset_name = {
                            let res = femtoflatbuffers::ComponentEncode::value_encode(&self.#field_name, encoder)?;
                            if let Some((global_offset, tmp_value)) = res {
                                Some(((global_offset - #start_offset_ident) as u16, tmp_value))
                            }
                            else {
                                None
                            }
                        };
                    });
                    offsets_encode.push(quote! {
                        encoder.encode_u16(#offset_name.as_ref().map(|x| x.0.clone()).unwrap_or(0))?;
                    });
                    post_encodes.push(quote! {
                        if let Some((_, tmp_value)) = #offset_name {
                            femtoflatbuffers::ComponentEncode::post_encode(&self.#field_name, encoder, tmp_value)?;
                        }
                    });
                }
                inner_do_table_encode(start_offset_ident, &fields_encode, &offsets_encode, &post_encodes)
            }
            _ => panic!("Only named fields are supported"),
        }
    } else {
        panic!("Only structs are supported");
    }
}

fn inner_do_decode_table(type_name: Ident, root_offset_ident: Ident, offset_calcs: &[TokenStream], struct_populations: &[TokenStream]) -> TokenStream {
    quote! {
        let vtable_offset = ((#root_offset_ident as i32) - decoder.decode_i32(#root_offset_ident)?) as u32;
        let vtable_size = decoder.decode_u16(vtable_offset)?;
        let table_size = decoder.decode_u16(vtable_offset + 2)?;
        #(#offset_calcs)*
        let res = #type_name {
            #(#struct_populations,)*
        };
        Ok(res)
    }
}

fn do_decode_table(type_name: Ident, data: &Data, root_offset_ident: Ident) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        match data.fields {
            syn::Fields::Named(ref fields) => {
                let mut offset_calcs = Vec::new();
                let mut struct_populations = Vec::new();
                let mut offset = 4u32;
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap();
                    let offset_name = format_ident!("{}_offset_blah", field_name);
                    offset_calcs.push(quote! {
                        let #offset_name = {
                            let val = decoder.decode_u16(vtable_offset + #offset)?;
                            if val == 0 {
                                None
                            } else {
                                Some(val)
                            }
                        };
                    });
                    offset += 2;
                    struct_populations.push(quote! {
                        #field_name: femtoflatbuffers::ComponentDecode::value_decode(&decoder, #offset_name.map(|x| x as u32 + #root_offset_ident))?
                    });
                }
                inner_do_decode_table(type_name, root_offset_ident, &offset_calcs, &struct_populations)
            }
            _ => panic!("Only named fields are supported"),
        }
    } else {
        panic!("Only structs are supported");
    }
}

enum TestTest {
    TEST1(u32),
    TEST2(u32, u32)
}

fn test(test: TestTest) {
    match test {
        TestTest::TEST1(a) => println!("{:?}", a),
        TestTest::TEST2(a, ..) => println!("{:?}", a)
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
        let mut fixit_enum_ident = format_ident!("FixItEnum{}", name);
        let mut tmp_value_enum_arms = vec![];
        let mut value_encode_match_cases = vec![];
        let mut post_encode_match_cases = vec![];
        let mut decode_match_cases = vec![];
        for variant in &data.variants {
            let variant_ident = variant.ident.clone();
            if variant_id == 0 {
                value_encode_match_cases.push(quote!{
                    #name::#variant_ident => {
                        let res = encoder.encode_u8(#variant_id)?;
                        (res, None)
                    }
                });
            }
            else {
                let variant_field = variant.fields.iter().next().unwrap();
                let variant_type = &variant_field.ty;
                let enum_arm_ident = format_ident!("{}_arm", variant_ident);
                tmp_value_enum_arms.push(quote!{
                    #enum_arm_ident(<#variant_type as femtoflatbuffers::ComponentEncode>::TmpValue)
                });
                value_encode_match_cases.push(quote!{
                    #name::#variant_ident(field, ..) => {
                        let res = encoder.encode_u8(#variant_id)?;
                        let value_res = femtoflatbuffers::ComponentEncode::value_encode(field, encoder)?;
                        (res, Some(#fixit_enum_ident::#enum_arm_ident(value_res.unwrap().1)))
                    }
               });
                post_encode_match_cases.push(quote!{
                    (#name::#variant_ident(field, ..), Some(#fixit_enum_ident::#enum_arm_ident(tmp_value))) => {
                        femtoflatbuffers::ComponentEncode::post_encode(field, encoder, tmp_value)?;
                    }
                });
                decode_match_cases.push(quote!{
                    #variant_id => {
                        let offset = decoder.decode_u32(offset)?;
                        femtoflatbuffers::ComponentDecode::value_decode(decoder, Some(offset + 4)).map(|field| #name::#variant_ident(field))
                    }
                });
            }
            variant_id += 1;
        }
        let expanded = quote! {
            enum #fixit_enum_ident {
                #(#tmp_value_enum_arms,)*
            }
            impl #impl_generics femtoflatbuffers::ComponentEncode for #name #ty_generics #where_clause {
                type TmpValue = Option<#fixit_enum_ident>;
                fn value_encode(&self, encoder: &mut femtoflatbuffers::Encoder) -> Result<Option<(u32, Self::TmpValue)>, femtoflatbuffers::EncodeError> {
                    Ok(Some(match self {
                        #(#value_encode_match_cases)*
                    }))
                }
                fn post_encode(&self, encoder: &mut femtoflatbuffers::Encoder, tmp_value: Self::TmpValue) -> Result<(), femtoflatbuffers::EncodeError> {
                    match (self, tmp_value) {
                        #(#post_encode_match_cases)*
                        _ => {}
                    }
                    Ok(())
                }
            }
            impl #impl_generics femtoflatbuffers::ComponentDecode for #name #ty_generics #where_clause {
                fn value_decode(decoder: &femtoflatbuffers::Decoder, offset: Option<u32>) -> Result<Self, femtoflatbuffers::DecodeError> {
                    if let Some(offset) = offset {
                        match decoder.decode_u8(offset)? {
                            #(#decode_match_cases,)*
                            _ => {
                                Err(femtoflatbuffers::DecodeError::InvalidData)
                            }
                        }
                    }
                    else {
                        Err(femtoflatbuffers::DecodeError::InvalidData)
                    }
                }
                fn table_value_size(decoder: &femtoflatbuffers::Decoder, table_value_global_offset: Option<u32>) -> Result<usize, femtoflatbuffers::DecodeError> {
                    if let Some(offset) = table_value_global_offset {
                        if decoder.decode_u8(offset)? == 0 {
                            Ok(4)
                        }
                        else {
                            Ok(8)
                        }
                    }
                    else {
                        Ok(0)
                    }
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
