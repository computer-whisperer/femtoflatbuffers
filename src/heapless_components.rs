use crate::{heapless_components, ComponentDecode, ComponentEncode, DecodeError, Decoder, EncodeError, Encoder};

#[cfg(feature = "heapless")]
impl <T: ComponentEncode, const N: usize> ComponentEncode for heapless::vec::Vec<T, N> {
    type WorkingValue = Option<(u32, u32)>;

    fn value_encode(&self, encoder: &mut Encoder, table_start: u32) -> Result<Self::WorkingValue, EncodeError> {
        if !self.is_empty() {
            let value_offset = encoder.encode_i32(0)?;
            Ok(Some((table_start, value_offset)))
        }
        else {
            Ok(None)
        }
    }

    fn vtable_encode(&self, encoder: &mut Encoder, _vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        match working_value {
            Some((table_start, value_offset)) => {
                encoder.encode_u16((value_offset - table_start) as u16)?;
                Ok(())
            }
            None => {
                encoder.encode_u16(0)?;
                Ok(())
            }
        }
    }

    fn post_encode(&self, encoder: &mut Encoder, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        if let Some((_table_start, value_offset)) = working_value {
            let global_list_start = encoder.encode_u32(self.len() as u32)?;

            let mut working_values = heapless::vec::Vec::<_, N>::new();
            for x in self.iter() {
                let working_value = x.value_encode(encoder, global_list_start)?;
                working_values.push(working_value);
            }

            for (working_value, x) in working_values.into_iter().zip(self.iter()) {
                x.post_encode(encoder, &working_value)?;
            }

            encoder.encode_i32_at(*value_offset, (global_list_start - value_offset) as i32)?;
            Ok(())
        }
        else {
            Ok(())
        }
    }
}

#[cfg(feature = "heapless")]
impl <T: ComponentDecode, const N: usize> ComponentDecode for heapless::vec::Vec<T, N> {
    type WorkingValue = Option<T::VectorWorkingValue>;
    type VectorWorkingValue = (); // Nested vectors are not supported by flatbuffers

    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_value = decoder.decode_u16(vtable_entry)?;
        if vtable_value == 0 {
            Ok((None, vtable_entry+2))
        }
        else {
            let (working_value, next_offset) = T::vector_vtable_decode(decoder, table_start, vtable_entry)?;
            Ok((Some(working_value), next_offset))
        }
    }
    fn value_decode(decoder: &Decoder, working_value: &Self::WorkingValue) -> Result<Self, DecodeError> {
        if let Some(working_value) = working_value {
            let vector_len = T::vector_len_decode(decoder, working_value)?;
            let mut result = heapless::vec::Vec::new();
            for idx in 0..vector_len.min(N) {
                result.push(T::vector_value_decode(decoder, working_value, idx)?);
            }
            Ok(result)
        } else {
            Ok(heapless::vec::Vec::new())
        }
    }

    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError>
    where
        Self: Sized
    {
        Err(DecodeError::InvalidData)
    }
}

#[cfg(feature = "heapless")]
impl <const N: usize> ComponentEncode for heapless::string::String<N> {
    type WorkingValue = Option<(u32, u32)>;

    fn value_encode(&self, encoder: &mut Encoder, table_start: u32) -> Result<Self::WorkingValue, EncodeError> {
        if !self.is_empty() {
            let value_offset = encoder.encode_i32(0)?;
            Ok(Some((table_start, value_offset)))
        }
        else {
            Ok(None)
        }
    }

    fn vtable_encode(&self, encoder: &mut Encoder, _vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        match working_value {
            Some((table_start, value_offset)) => {
                encoder.encode_u16((value_offset - table_start) as u16)?;
                Ok(())
            }
            None => {
                encoder.encode_u16(0)?;
                Ok(())
            }
        }
    }

    fn post_encode(&self, encoder: &mut Encoder, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        if let Some((_table_start, value_offset)) = working_value {
            let global_list_start = encoder.encode_u32(self.len() as u32)?;

            for x in self.iter() {
                encoder.encode_u8(x)?;
            }
            encoder.encode_u8(0)?;

            encoder.encode_i32_at(*value_offset, (global_list_start - value_offset) as i32)?;
            Ok(())
        }
        else {
            Ok(())
        }
    }
}

#[cfg(feature = "heapless")]
impl <const N: usize> ComponentDecode for heapless::string::String<N> {
    type WorkingValue = (u32, u16);
    type VectorWorkingValue = (); // Nested vectors are not supported by flatbuffers

    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_value = decoder.decode_u16(vtable_entry)?;
        Ok(((table_start, vtable_value), vtable_entry+2))
    }
    fn value_decode(decoder: &Decoder, working_value: &Self::WorkingValue) -> Result<Self, DecodeError> {
        if working_value.1 == 0 {
            Ok(heapless::string::String::new())
        }
        else {
            let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32 + working_value.1 as i32) as u32;
            let vector_len = decoder.decode_u32(vector_offset)?;
            let mut result = heapless::string::String::new();
            for idx in 0..vector_len.min(N as u32) {
                if let Err(_) =result.push(decoder.decode_u8(vector_offset + 4 + idx as u32)? as char) {
                    return Err(DecodeError::CollectionOverflow);
                }
            }
            Ok(result)
        }
    }

    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError>
    where
        Self: Sized
    {
        Err(DecodeError::InvalidData)
    }
}
