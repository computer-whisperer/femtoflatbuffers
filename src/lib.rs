pub mod table;

pub use femtoflatbuffers_derive::Table;

#[derive(thiserror::Error, Debug)]
pub enum EncodeError {
    #[error("Not enough space in buffer")]
    OutOfSpace
}

#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error("Invalid data")]
    InvalidData
}

pub struct Encoder<'a> {
    buffer: &'a mut [u8],
    used_bytes: usize
}

impl<'a> Encoder<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {buffer, used_bytes: 0}
    }
    pub fn used_bytes(&self) -> u32 {
        self.used_bytes as u32
    }
    pub fn done(self) -> &'a [u8] {
        &self.buffer[..self.used_bytes as usize]
    }

    pub fn pad_to_align(&mut self, align: usize) -> Result<(), EncodeError> {
        let padding = (align - self.used_bytes % align) % align;
        if self.used_bytes + padding > self.buffer.len() {
            return Err(EncodeError::OutOfSpace);
        }
        self.used_bytes += padding;
        Ok(())
    }

    pub fn encode_u32(&mut self, value: u32) -> Result<u32, EncodeError> {
        self.pad_to_align(4)?;
        if self.buffer.len() - self.used_bytes < 4 {
            return Err(EncodeError::OutOfSpace);
        }
        let offset = self.used_bytes as u32;
        self.buffer[self.used_bytes..self.used_bytes+4].copy_from_slice(&value.to_le_bytes());
        self.used_bytes += 4;
        Ok(offset)
    }

    pub fn encode_i32(&mut self, value: i32) -> Result<u32, EncodeError> {
        self.encode_u32(value as u32)
    }

    pub fn encode_u32_at(&mut self, offset: u32, value: u32) -> Result<(), EncodeError> {
        self.buffer[offset as usize..offset as usize+4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn encode_i32_at(&mut self, offset: u32, value: i32) -> Result<(), EncodeError> {
        self.encode_u32_at(offset, value as u32)
    }

    pub fn encode_u16(&mut self, value: u16) -> Result<u32, EncodeError> {
        self.pad_to_align(2)?;
        if self.buffer.len() - self.used_bytes < 2 {
            return Err(EncodeError::OutOfSpace);
        }
        let offset = self.used_bytes as u32;
        self.buffer[self.used_bytes..self.used_bytes+2].copy_from_slice(&value.to_le_bytes());
        self.used_bytes += 2;
        Ok(offset)
    }

    pub fn encode_i16(&mut self, value: i16) -> Result<u32, EncodeError> {
        self.encode_u16(value as u16)
    }

    pub fn encode_u16_at(&mut self, offset: u32, value: u16) -> Result<(), EncodeError> {
        self.buffer[offset as usize..offset as usize+2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}



pub struct Decoder<'a> {
    buffer: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {buffer}
    }

    pub fn decode_u32(&self, offset: u32) -> Result<u32, DecodeError> {
        if offset + 4 > self.buffer.len() as u32 {
            Err(DecodeError::InvalidData)
        } else {
            Ok(u32::from_le_bytes(
                self.buffer[offset as usize..offset as usize + 4].try_into().unwrap()
            ))
        }
    }

    pub fn decode_i32(&self, offset: u32) -> Result<i32, DecodeError> {
        self.decode_u32(offset).map(|x| x as i32)
    }

    pub fn decode_u16(&self, offset: u32) -> Result<u16, DecodeError> {
        if offset + 2 > self.buffer.len() as u32 {
            Err(DecodeError::InvalidData)
        } else {
            Ok(u16::from_le_bytes(
                self.buffer[offset as usize..offset as usize + 2].try_into().unwrap()
            ))
        }
    }
}

pub trait ComponentEncode {
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<u32>, EncodeError>;
    fn vtable_entry_encode(&self, encoder: &mut Encoder, value_offset: Option<u16>) -> Result<(), EncodeError>;
    fn post_encode(&self, _encoder: &mut Encoder) -> Result<Option<u32>, EncodeError> {Ok(None)}
}

pub trait ComponentDecode {
    fn value_decode(decoder: &Decoder, table_start_offset: u32, value_offset: Option<u16>) -> Result<Self, DecodeError> where Self: Sized;
}

impl ComponentEncode for u32 {
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<u32>, EncodeError> {
        Ok(Some(encoder.encode_u32(*self)?))
    }

    fn vtable_entry_encode(&self, encoder: &mut Encoder, value_offset: Option<u16>) -> Result<(), EncodeError> {
        encoder.encode_u16(value_offset.unwrap_or(0))?;
        Ok(())
    }
}

impl ComponentDecode for u32 {
    fn value_decode(decoder: &Decoder, table_start_offset: u32, value_offset: Option<u16>) -> Result<Self, DecodeError> {
        if let Some(value_offset) = value_offset {
            Ok(decoder.decode_u32(table_start_offset + value_offset as u32)?)
        }
        else {
            Err(DecodeError::InvalidData)
        }
    }
}

impl <T: ComponentEncode> ComponentEncode for Option<T> {
    fn value_encode(&self, encoder: &mut Encoder) -> Result<Option<u32>, EncodeError> {
        match self {
            Some(x) => x.value_encode(encoder),
            None => Ok(None)
        }
    }
    fn vtable_entry_encode(&self, encoder: &mut Encoder, value_offset: Option<u16>) -> Result<(), EncodeError> {
        match self {
            Some(x) => x.vtable_entry_encode(encoder, value_offset),
            None => Ok(())
        }
    }
    fn post_encode(&self, encoder: &mut Encoder) -> Result<Option<u32>, EncodeError> {
        match self {
            Some(x) => x.post_encode(encoder),
            None => Ok(None)
        }
    }
}

impl <T: ComponentDecode> ComponentDecode for Option<T> {
    fn value_decode(decoder: &Decoder, table_start_offset: u32, value_offset: Option<u16>) -> Result<Self, DecodeError> {
        match value_offset {
            Some(value_offset) => Ok(Some(T::value_decode(decoder, table_start_offset, Some(value_offset))?)),
            None => Ok(None)
        }
    }
}