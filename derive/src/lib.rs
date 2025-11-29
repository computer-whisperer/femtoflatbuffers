extern crate proc_macro;

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
pub fn flatbuffers_struct_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
            fn table_value_size(table_value_global_offset: Option<u32>) -> usize {
                if let Some(_) = table_value_global_offset {
                    4
                }
                else {
                    0
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
                        encoder.encode_u16(#offset_name.map(|x| x.0).unwrap_or(0))?;
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

#[cfg(test)]
mod tests {}
