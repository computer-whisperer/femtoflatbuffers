use crate::{ComponentDecode, ComponentEncode, DecodeError, Decoder, EncodeError, Encoder};

#[cfg(feature = "heapless")]
impl<T: ComponentEncode, const N: usize> ComponentEncode for heapless::vec::Vec<T, N> {
    type WorkingValue = Option<(u32, u32)>;

    fn value_encode(
        &self,
        encoder: &mut Encoder,
        table_start: u32,
    ) -> Result<Self::WorkingValue, EncodeError> {
        if !self.is_empty() {
            let value_offset = encoder.encode_i32(0)?;
            Ok(Some((table_start, value_offset)))
        } else {
            Ok(None)
        }
    }

    fn vtable_encode(
        &self,
        encoder: &mut Encoder,
        _vtable_start: u32,
        working_value: &Self::WorkingValue,
    ) -> Result<(), EncodeError> {
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

    fn post_encode(
        &self,
        encoder: &mut Encoder,
        working_value: &Self::WorkingValue,
    ) -> Result<(), EncodeError> {
        if let Some((_table_start, value_offset)) = working_value {
            // Position the length prefix so the elements land at their natural
            // alignment immediately after it (readers assume `start + 4`).
            encoder.pad_for_vector(<T as ComponentEncode>::alignment())?;
            let global_list_start = encoder.encode_u32(self.len() as u32)?;

            let mut working_values = heapless::vec::Vec::<_, N>::new();
            for x in self.iter() {
                let working_value = x.value_encode(encoder, global_list_start)?;
                // Cannot overflow: `working_values` and `self` share capacity N.
                let _ = working_values.push(working_value);
            }

            for (working_value, x) in working_values.into_iter().zip(self.iter()) {
                x.post_encode(encoder, &working_value)?;
            }

            encoder.encode_i32_at(*value_offset, (global_list_start - value_offset) as i32)?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "heapless")]
impl<T: ComponentDecode, const N: usize> ComponentDecode for heapless::vec::Vec<T, N> {
    type WorkingValue = Option<T::VectorWorkingValue>;
    type VectorWorkingValue = (); // Nested vectors are not supported by flatbuffers

    fn vtable_decode(
        decoder: &Decoder,
        table_start: u32,
        vtable_entry: u32,
    ) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_value = decoder.vtable_entry_at(table_start, vtable_entry)?;
        if vtable_value == 0 {
            Ok((None, vtable_entry + 2))
        } else {
            let (working_value, next_offset) =
                T::vector_vtable_decode(decoder, table_start, vtable_entry)?;
            Ok((Some(working_value), next_offset))
        }
    }
    fn value_decode(
        decoder: &Decoder,
        working_value: &Self::WorkingValue,
    ) -> Result<Self, DecodeError> {
        if let Some(working_value) = working_value {
            let vector_len = T::vector_len_decode(decoder, working_value)?;
            let mut result = heapless::vec::Vec::new();
            for idx in 0..vector_len.min(N) {
                let value = T::vector_value_decode(decoder, working_value, idx)?;
                // Cannot overflow: the loop is bounded by `.min(N)`.
                let _ = result.push(value);
            }
            Ok(result)
        } else {
            Ok(heapless::vec::Vec::new())
        }
    }

    fn vector_vtable_decode(
        _decoder: &Decoder,
        _table_start: u32,
        _vtable_entry: u32,
    ) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_len_decode(
        _decoder: &Decoder,
        _working_value: &Self::VectorWorkingValue,
    ) -> Result<usize, DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_value_decode(
        _decoder: &Decoder,
        _working_value: &Self::VectorWorkingValue,
        _idx: usize,
    ) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Err(DecodeError::InvalidData)
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> ComponentEncode for heapless::string::String<N> {
    type WorkingValue = Option<(u32, u32)>;

    fn value_encode(
        &self,
        encoder: &mut Encoder,
        table_start: u32,
    ) -> Result<Self::WorkingValue, EncodeError> {
        if !self.is_empty() {
            let value_offset = encoder.encode_i32(0)?;
            Ok(Some((table_start, value_offset)))
        } else {
            Ok(None)
        }
    }

    fn vtable_encode(
        &self,
        encoder: &mut Encoder,
        _vtable_start: u32,
        working_value: &Self::WorkingValue,
    ) -> Result<(), EncodeError> {
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

    fn post_encode(
        &self,
        encoder: &mut Encoder,
        working_value: &Self::WorkingValue,
    ) -> Result<(), EncodeError> {
        if let Some((_table_start, value_offset)) = working_value {
            let global_list_start = encoder.encode_u32(self.len() as u32)?;

            for x in self.as_bytes() {
                encoder.encode_u8(*x)?;
            }
            encoder.encode_u8(0)?;

            encoder.encode_i32_at(*value_offset, (global_list_start - value_offset) as i32)?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "heapless")]
impl<const N: usize> ComponentDecode for heapless::string::String<N> {
    type WorkingValue = (u32, u16);
    type VectorWorkingValue = (); // Nested vectors are not supported by flatbuffers

    fn vtable_decode(
        decoder: &Decoder,
        table_start: u32,
        vtable_entry: u32,
    ) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_value = decoder.vtable_entry_at(table_start, vtable_entry)?;
        Ok(((table_start, vtable_value), vtable_entry + 2))
    }
    fn value_decode(
        decoder: &Decoder,
        working_value: &Self::WorkingValue,
    ) -> Result<Self, DecodeError> {
        if working_value.1 == 0 {
            Ok(heapless::string::String::new())
        } else {
            let field_offset = Decoder::offset_add(working_value.0, working_value.1 as u32)?;
            let vector_offset = decoder.follow_offset(field_offset)?;
            let vector_len = decoder.decode_u32(vector_offset)?;
            // The capacity is bounded by the const N, so no work-budget check is
            // needed; the loop is capped at N regardless of the claimed length.
            let mut result = heapless::string::String::new();
            for idx in 0..vector_len.min(N as u32) {
                let byte_offset = Decoder::vector_element_offset(vector_offset, idx as usize, 1)?;
                if result
                    .push(decoder.decode_u8(byte_offset)? as char)
                    .is_err()
                {
                    return Err(DecodeError::CollectionOverflow);
                }
            }
            Ok(result)
        }
    }

    fn vector_vtable_decode(
        _decoder: &Decoder,
        _table_start: u32,
        _vtable_entry: u32,
    ) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_len_decode(
        _decoder: &Decoder,
        _working_value: &Self::VectorWorkingValue,
    ) -> Result<usize, DecodeError> {
        Err(DecodeError::InvalidData)
    }

    fn vector_value_decode(
        _decoder: &Decoder,
        _working_value: &Self::VectorWorkingValue,
        _idx: usize,
    ) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Err(DecodeError::InvalidData)
    }
}
