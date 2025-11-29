use crate::{DecodeError, Decoder, EncodeError, Encoder};

pub trait ComponentEncode {
    type TmpValue;
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<(u32, Self::TmpValue)>, EncodeError>;
    fn post_encode(&self, _encoder: &mut Encoder, _tmp_value: Self::TmpValue) -> Result<(), EncodeError> {Ok(())}
}

pub trait ComponentDecode {
    fn value_decode(decoder: &Decoder, table_value_global_offset: Option<u32>) -> Result<Self, DecodeError> where Self: Sized;
    fn table_value_size(table_value_global_offset: Option<u32>) -> usize;
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

impl <T: PrimitiveComponent> ComponentEncode for T {
    type TmpValue = ();
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<(u32, Self::TmpValue)>, EncodeError> {
        Ok(Some((self.do_encode(encoder)?, ())))
    }
}

impl <T: PrimitiveComponent> ComponentDecode for T {
    fn value_decode(decoder: &Decoder, table_value_global_offset: Option<u32>) -> Result<Self, DecodeError> {
        if let Some(value_offset) = table_value_global_offset {
            Ok(T::do_decode(decoder, value_offset)?)
        }
        else {
            Err(DecodeError::InvalidData)
        }
    }
    fn table_value_size(table_value_global_offset: Option<u32>) -> usize {
        if let Some(_) = table_value_global_offset {
            T::size()
        }
        else {
            0
        }
    }
}

impl <T: ComponentEncode> ComponentEncode for Option<T> {
    type TmpValue = T::TmpValue;
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<(u32, Self::TmpValue)>, EncodeError> {
        match self {
            Some(x) => x.value_encode(encoder),
            None => Ok(None)
        }
    }
    fn post_encode(&self, encoder: &mut Encoder, tmp_value: Self::TmpValue) -> Result<(), EncodeError> {
        match self {
            Some(x) => x.post_encode(encoder, tmp_value),
            None => Ok(())
        }
    }
}

impl <T: ComponentDecode> ComponentDecode for Option<T> {
    fn value_decode(decoder: &Decoder, table_value_global_offset: Option<u32>) -> Result<Self, DecodeError> {
        match table_value_global_offset {
            Some(table_value_global_offset) => Ok(Some(T::value_decode(decoder, Some(table_value_global_offset))?)),
            None => Ok(None)
        }
    }
    fn table_value_size(table_value_global_offset: Option<u32>) -> usize {
        if let Some(_) = table_value_global_offset {
            T::table_value_size(Some(0))
        }
        else {
            0
        }
    }
}

#[cfg(feature = "alloc")]
impl <T: ComponentEncode> ComponentEncode for alloc::vec::Vec<T> {
    type TmpValue = u32;

    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<(u32, Self::TmpValue)>, EncodeError> {
        if self.is_empty() {
            Ok(None)
        }
        else {
            let value_offset = encoder.encode_i32(0)?;
            Ok(Some((value_offset, value_offset)))
        }
    }

    fn post_encode(&self, encoder: &mut Encoder, tmp_value: Self::TmpValue) -> Result<(), EncodeError> {
        let global_list_start = encoder.encode_u32(self.len() as u32)?;

        let mut tmp_values = alloc::vec::Vec::with_capacity(self.len());
        for x in self.iter() {
            let (_offset, tmp_value) = x.value_encode(encoder)?.ok_or(EncodeError::InvalidStructure)?;
            tmp_values.push(tmp_value);
        }

        for (tmp_value, x) in tmp_values.into_iter().zip(self.iter()) {
            x.post_encode(encoder, tmp_value)?;
        }

        let table_entry_offset = tmp_value;
        encoder.encode_i32_at(table_entry_offset, (global_list_start - table_entry_offset) as i32)?;
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl <T: ComponentDecode> ComponentDecode for alloc::vec::Vec<T> {
    fn value_decode(decoder: &Decoder, table_value_global_offset: Option<u32>) -> Result<Self, DecodeError> {
        if let Some(table_value_global_offset) = table_value_global_offset {
            let list_global_offset = (table_value_global_offset as i32 + decoder.decode_i32(table_value_global_offset)?) as u32;
            let list_length = decoder.decode_u32(list_global_offset)?;
            let mut working_offset = list_global_offset + 4;
            let mut result = alloc::vec::Vec::with_capacity(list_length as usize);
            for _ in 0..list_length {
                result.push(T::value_decode(decoder, Some(working_offset))?);
                working_offset += T::table_value_size(Some(working_offset)) as u32;
            }
            Ok(result)
        } else {
            Ok(alloc::vec::Vec::new())
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