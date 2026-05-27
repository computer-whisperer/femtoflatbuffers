#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "heapless")]
mod heapless_components;

pub mod components;
pub mod table;

pub use components::{ComponentDecode, ComponentEncode};
pub use femtoflatbuffers_derive::{Table, Union};

#[derive(thiserror::Error, Debug)]
pub enum EncodeError {
    #[error("Not enough space in buffer")]
    OutOfSpace,
    #[error("Invalid structure")]
    InvalidStructure,
}

#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error("Invalid data")]
    InvalidData,
    #[error("Unsupported Feature")]
    UnsupportedFeature,
    #[error("Collection Overflow")]
    CollectionOverflow,
}

pub struct Encoder<'a> {
    buffer: &'a mut [u8],
    used_bytes: usize,
}

impl<'a> Encoder<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            used_bytes: 0,
        }
    }
    pub fn used_bytes(&self) -> u32 {
        self.used_bytes as u32
    }
    pub fn done(self) -> &'a [u8] {
        &self.buffer[..self.used_bytes]
    }

    pub fn pad_to_align(&mut self, align: usize) -> Result<(), EncodeError> {
        let padding = (align - self.used_bytes % align) % align;
        if self.used_bytes + padding > self.buffer.len() {
            return Err(EncodeError::OutOfSpace);
        }
        self.used_bytes += padding;
        Ok(())
    }

    pub fn encode_u64(&mut self, value: u64) -> Result<u32, EncodeError> {
        self.pad_to_align(8)?;
        if self.buffer.len() - self.used_bytes < 8 {
            return Err(EncodeError::OutOfSpace);
        }
        let offset = self.used_bytes as u32;
        self.buffer[self.used_bytes..self.used_bytes + 8].copy_from_slice(&value.to_le_bytes());
        self.used_bytes += 8;
        Ok(offset)
    }

    pub fn encode_i64(&mut self, value: i64) -> Result<u32, EncodeError> {
        self.encode_u64(value as u64)
    }

    pub fn encode_u32(&mut self, value: u32) -> Result<u32, EncodeError> {
        self.pad_to_align(4)?;
        if self.buffer.len() - self.used_bytes < 4 {
            return Err(EncodeError::OutOfSpace);
        }
        let offset = self.used_bytes as u32;
        self.buffer[self.used_bytes..self.used_bytes + 4].copy_from_slice(&value.to_le_bytes());
        self.used_bytes += 4;
        Ok(offset)
    }

    pub fn encode_i32(&mut self, value: i32) -> Result<u32, EncodeError> {
        self.encode_u32(value as u32)
    }

    pub fn encode_u32_at(&mut self, offset: u32, value: u32) -> Result<(), EncodeError> {
        self.buffer[offset as usize..offset as usize + 4].copy_from_slice(&value.to_le_bytes());
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
        self.buffer[self.used_bytes..self.used_bytes + 2].copy_from_slice(&value.to_le_bytes());
        self.used_bytes += 2;
        Ok(offset)
    }

    pub fn encode_i16(&mut self, value: i16) -> Result<u32, EncodeError> {
        self.encode_u16(value as u16)
    }

    pub fn encode_u16_at(&mut self, offset: u32, value: u16) -> Result<(), EncodeError> {
        self.buffer[offset as usize..offset as usize + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn encode_u8(&mut self, value: u8) -> Result<u32, EncodeError> {
        if self.buffer.len() - self.used_bytes < 1 {
            return Err(EncodeError::OutOfSpace);
        }
        let offset = self.used_bytes as u32;
        self.buffer[self.used_bytes] = value;
        self.used_bytes += 1;
        Ok(offset)
    }

    pub fn encode_i8(&mut self, value: i8) -> Result<u32, EncodeError> {
        self.encode_u8(value as u8)
    }

    pub fn encode_f32(&mut self, value: f32) -> Result<u32, EncodeError> {
        self.encode_u32(value.to_bits())
    }

    pub fn encode_f64(&mut self, value: f64) -> Result<u32, EncodeError> {
        self.encode_u64(value.to_bits())
    }
}

pub struct Decoder<'a> {
    buffer: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }

    pub fn decode_u64(&self, offset: u32) -> Result<u64, DecodeError> {
        if offset + 8 > self.buffer.len() as u32 {
            Err(DecodeError::InvalidData)
        } else {
            Ok(u64::from_le_bytes(
                self.buffer[offset as usize..offset as usize + 8]
                    .try_into()
                    .unwrap(),
            ))
        }
    }

    pub fn decode_i64(&self, offset: u32) -> Result<i64, DecodeError> {
        self.decode_u64(offset).map(|x| x as i64)
    }

    pub fn decode_u32(&self, offset: u32) -> Result<u32, DecodeError> {
        if offset + 4 > self.buffer.len() as u32 {
            Err(DecodeError::InvalidData)
        } else {
            Ok(u32::from_le_bytes(
                self.buffer[offset as usize..offset as usize + 4]
                    .try_into()
                    .unwrap(),
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
                self.buffer[offset as usize..offset as usize + 2]
                    .try_into()
                    .unwrap(),
            ))
        }
    }

    pub fn decode_i16(&self, offset: u32) -> Result<i16, DecodeError> {
        self.decode_u16(offset).map(|x| x as i16)
    }

    pub fn decode_u8(&self, offset: u32) -> Result<u8, DecodeError> {
        if offset + 1 > self.buffer.len() as u32 {
            Err(DecodeError::InvalidData)
        } else {
            Ok(self.buffer[offset as usize])
        }
    }

    pub fn decode_i8(&self, offset: u32) -> Result<i8, DecodeError> {
        self.decode_u8(offset).map(|x| x as i8)
    }

    pub fn decode_f32(&self, offset: u32) -> Result<f32, DecodeError> {
        self.decode_u32(offset).map(f32::from_bits)
    }

    pub fn decode_f64(&self, offset: u32) -> Result<f64, DecodeError> {
        self.decode_u64(offset).map(f64::from_bits)
    }

    /// Read the vtable slot at absolute offset `vtable_entry` for the table at
    /// `table_start`. Returns `0` when the slot lies beyond that table's vtable,
    /// which is how a FlatBuffers writer signals a trailing field omitted
    /// because it held the default value. A stored `0` likewise means "absent".
    pub fn vtable_entry_at(&self, table_start: u32, vtable_entry: u32) -> Result<u16, DecodeError> {
        let vtable_offset = (table_start as i32 - self.decode_i32(table_start)?) as u32;
        let vtable_size = self.decode_u16(vtable_offset)?;
        if vtable_entry + 2 <= vtable_offset + vtable_size as u32 {
            self.decode_u16(vtable_entry)
        } else {
            Ok(0)
        }
    }
}
