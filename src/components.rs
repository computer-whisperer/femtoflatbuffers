use crate::{DecodeError, Decoder, EncodeError, Encoder};

pub trait ComponentEncode {
    type WorkingValue;
    fn value_encode(&self, encoder: &mut Encoder, table_start: u32) -> Result<Self::WorkingValue, EncodeError>;
    fn vtable_encode(&self, encoder: &mut Encoder, vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), EncodeError>;
    fn post_encode(&self, _encoder: &mut Encoder, _working_value: &Self::WorkingValue) -> Result<(), EncodeError> {Ok(())}
}

pub trait ComponentDecode {
    type WorkingValue;
    type VectorWorkingValue;
    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError>;
    fn value_decode(decoder: &Decoder, working_value: &Self::WorkingValue) -> Result<Self, DecodeError> where Self: Sized;
    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError>;
    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError>;
    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError> where Self: Sized;
}

pub trait PrimitiveComponent {
    fn alignment() -> usize;
    fn size() -> usize;
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError>;
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> where Self: Sized;
}

impl PrimitiveComponent for u32 {
    fn alignment() -> usize {4}
    fn size() -> usize {4}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_u32(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_u32(offset)?)}
}

impl PrimitiveComponent for u64 {
    fn alignment() -> usize {8}
    fn size() -> usize {8}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_u64(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_u64(offset)?)}
}

impl PrimitiveComponent for i64 {
    fn alignment() -> usize {8}
    fn size() -> usize {8}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_i64(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_i64(offset)?)}
}

impl PrimitiveComponent for i32 {
    fn alignment() -> usize {4}
    fn size() -> usize {4}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_i32(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_i32(offset)?)}
}

impl PrimitiveComponent for u16 {
    fn alignment() -> usize {2}
    fn size() -> usize {2}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_u16(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_u16(offset)?)}
}

impl PrimitiveComponent for i16 {
    fn alignment() -> usize {2}
    fn size() -> usize {2}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_i16(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_i16(offset)?)}
}

impl PrimitiveComponent for u8 {
    fn alignment() -> usize {1}
    fn size() -> usize {1}
    fn do_encode(&self, encoder: &mut Encoder) -> Result<u32, EncodeError> {encoder.encode_u8(*self)}
    fn do_decode(decoder: &Decoder, offset: u32) -> Result<Self, DecodeError> {Ok(decoder.decode_u8(offset)?)}
}

impl <T: PrimitiveComponent> ComponentEncode for T {
    type WorkingValue = (u32, u32);
    fn value_encode(&self, encoder: &mut Encoder, table_start: u32) -> Result<Self::WorkingValue, EncodeError> {
        let value_offset = self.do_encode(encoder)?;
        Ok((table_start, value_offset))
    }
    fn vtable_encode(&self, encoder: &mut Encoder, _vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        encoder.encode_u16((working_value.1 - working_value.0) as u16)?;
        Ok(())
    }
}

impl <T: PrimitiveComponent> ComponentDecode for T {
    type WorkingValue = (u32, u16);
    type VectorWorkingValue = Self::WorkingValue;
    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
        Ok(((table_start, vtable_entry_value), vtable_entry+2))
    }
    fn value_decode(decoder: &Decoder, working_value: &Self::WorkingValue) -> Result<Self, DecodeError> {
        Ok(T::do_decode(decoder, working_value.0 + working_value.1 as u32)?)
    }
    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
        Ok(((table_start, vtable_entry_value), vtable_entry+2))
    }
    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError> {
        let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32 + working_value.1 as i32) as u32;
        Ok(decoder.decode_u32(vector_offset)? as usize)
    }
    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError>
    where
        Self: Sized
    {
        let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32 + working_value.1 as i32) as u32;
        T::do_decode(decoder, (vector_offset+4) + (idx*Self::size()) as u32)
    }
}

impl <T: ComponentEncode> ComponentEncode for Option<T> {
    type WorkingValue = Option<T::WorkingValue>;
    fn value_encode(&self, encoder: &mut Encoder, table_start: u32) -> Result<Self::WorkingValue, EncodeError> {
        match self {
            Some(x) => Ok(Some(x.value_encode(encoder, table_start)?)),
            None => Ok(None)
        }
    }
    fn vtable_encode(&self, encoder: &mut Encoder, vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        match (self, working_value) {
            (Some(inner_self), Some(inner_working_value)) => {
                inner_self.vtable_encode(encoder, vtable_start, inner_working_value)?;
                Ok(())
            }
            (None, None) => {
                encoder.encode_u16(0)?;
                Ok(())
            }
            _ => {
                Err(EncodeError::InvalidStructure)
            }
        }
    }
    fn post_encode(&self, encoder: &mut Encoder, working_value: &Self::WorkingValue) -> Result<(), EncodeError> {
        match (self, working_value) {
            (Some(inner_self), Some(working_value)) => {
                inner_self.post_encode(encoder, working_value)?;
                Ok(())
            }
            (None, None) => {
                Ok(())
            }
            _ => {
                Err(EncodeError::InvalidStructure)
            }
        }
    }
}

impl <T: ComponentDecode> ComponentDecode for Option<T> {
    type WorkingValue = Option<T::WorkingValue>;
    type VectorWorkingValue = Option<T::VectorWorkingValue>;
    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let value = decoder.decode_u16(vtable_entry)?;
        match value {
            0 => Ok((None, vtable_entry+2)),
            _ => {
                let (working_value, next_offset) = T::vtable_decode(decoder, table_start, vtable_entry)?;
                Ok((Some(working_value), next_offset))
            }
        }
    }
    fn value_decode(decoder: &Decoder, working_value: &Self::WorkingValue) -> Result<Self, DecodeError> {
        match working_value {
            Some(working_value_inner) => Ok(Some(T::value_decode(decoder, working_value_inner)?)),
            None => Ok(None)
        }
    }
    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        let value = decoder.decode_u16(vtable_entry)?;
        match value {
            0 => Ok((None, vtable_entry+2)),
            _ => {
                let (working_value, next_offset) = T::vector_vtable_decode(decoder, table_start, vtable_entry)?;
                Ok((Some(working_value), next_offset))
            }
        }
    }
    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError> {
        match working_value {
            Some(working_value_inner) => {
                T::vector_len_decode(decoder, working_value_inner)
            }
            None => {
                Ok(0)
            }
        }
    }
    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        match working_value {
            Some(working_value_inner) => {
                Ok(Some(T::vector_value_decode(decoder, working_value_inner, idx)?))
            }
            None => {
                Err(DecodeError::InvalidData)
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl <T: ComponentEncode> ComponentEncode for alloc::vec::Vec<T> {
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

            let mut working_values = alloc::vec::Vec::with_capacity(self.len());
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

#[cfg(feature = "alloc")]
impl <T: ComponentDecode> ComponentDecode for alloc::vec::Vec<T> {
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
            let mut result = alloc::vec::Vec::with_capacity(vector_len as usize);
            for idx in 0..vector_len {
                result.push(T::vector_value_decode(decoder, working_value, idx)?);
            }
            Ok(result)
        } else {
            Ok(alloc::vec::Vec::new())
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
