#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "heapless")]
mod heapless_components;

pub mod components;
pub mod table;

use core::cell::Cell;

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
    #[error("Resource limit exceeded")]
    ResourceLimit,
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
    /// Current table-nesting depth; bounds recursion (see [`Decoder::enter_nested`]).
    depth: Cell<u32>,
    /// Remaining decode "work" units; bounds total tables + vector elements so a
    /// crafted buffer cannot amplify into unbounded work/allocation.
    budget: Cell<u32>,
}

/// Maximum nesting depth of tables/unions before decoding bails out.
const MAX_DEPTH: u32 = 64;

/// RAII token returned by [`Decoder::enter_nested`]; decrements the depth counter
/// on drop so the count is restored on every exit path, including `?` returns.
pub struct DepthGuard<'d> {
    depth: &'d Cell<u32>,
}

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }
}

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        // Each table and each vector element consumes at least one buffer byte in
        // a well-formed (non-aliased) buffer, so the byte length is a sound upper
        // bound on legitimate decode work.
        let budget = buffer.len().min(u32::MAX as usize) as u32;
        Self {
            buffer,
            depth: Cell::new(0),
            budget: Cell::new(budget),
        }
    }

    /// Read `LEN` bytes at `offset`, returning `InvalidData` rather than panicking
    /// for any out-of-range or overflowing offset.
    fn read_bytes<const LEN: usize>(&self, offset: u32) -> Result<[u8; LEN], DecodeError> {
        let start = offset as usize;
        let end = start.checked_add(LEN).ok_or(DecodeError::InvalidData)?;
        let slice = self
            .buffer
            .get(start..end)
            .ok_or(DecodeError::InvalidData)?;
        Ok(slice.try_into().unwrap())
    }

    pub fn decode_u64(&self, offset: u32) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.read_bytes::<8>(offset)?))
    }

    pub fn decode_i64(&self, offset: u32) -> Result<i64, DecodeError> {
        self.decode_u64(offset).map(|x| x as i64)
    }

    pub fn decode_u32(&self, offset: u32) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.read_bytes::<4>(offset)?))
    }

    pub fn decode_i32(&self, offset: u32) -> Result<i32, DecodeError> {
        self.decode_u32(offset).map(|x| x as i32)
    }

    pub fn decode_u16(&self, offset: u32) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.read_bytes::<2>(offset)?))
    }

    pub fn decode_i16(&self, offset: u32) -> Result<i16, DecodeError> {
        self.decode_u16(offset).map(|x| x as i16)
    }

    pub fn decode_u8(&self, offset: u32) -> Result<u8, DecodeError> {
        Ok(self.read_bytes::<1>(offset)?[0])
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

    /// Follow a forward `uoffset` stored at `position`: returns `position + i32@position`,
    /// range-checked to a valid `u32` so a malicious offset yields `InvalidData`
    /// rather than overflowing.
    pub fn follow_offset(&self, position: u32) -> Result<u32, DecodeError> {
        let rel = self.decode_i32(position)?;
        let target = position as i64 + rel as i64;
        if !(0..=u32::MAX as i64).contains(&target) {
            return Err(DecodeError::InvalidData);
        }
        Ok(target as u32)
    }

    /// Follow a backward `soffset` stored at `position`: returns `position - i32@position`,
    /// range-checked. Used to reach a table's vtable.
    pub fn follow_soffset(&self, position: u32) -> Result<u32, DecodeError> {
        let rel = self.decode_i32(position)?;
        let target = position as i64 - rel as i64;
        if !(0..=u32::MAX as i64).contains(&target) {
            return Err(DecodeError::InvalidData);
        }
        Ok(target as u32)
    }

    /// `base + delta`, range-checked to a valid `u32` offset.
    pub fn offset_add(base: u32, delta: u32) -> Result<u32, DecodeError> {
        base.checked_add(delta).ok_or(DecodeError::InvalidData)
    }

    /// Absolute offset of element `idx` (each `elem_size` bytes) within the vector
    /// body beginning at `vector_offset`, whose first 4 bytes are the length
    /// prefix. All arithmetic is checked, so any overflow yields `InvalidData`.
    pub fn vector_element_offset(
        vector_offset: u32,
        idx: usize,
        elem_size: usize,
    ) -> Result<u32, DecodeError> {
        let byte_idx = idx.checked_mul(elem_size).ok_or(DecodeError::InvalidData)?;
        let from_start = byte_idx.checked_add(4).ok_or(DecodeError::InvalidData)?;
        let delta = u32::try_from(from_start).map_err(|_| DecodeError::InvalidData)?;
        Self::offset_add(vector_offset, delta)
    }

    /// Read the vtable slot at absolute offset `vtable_entry` for the table at
    /// `table_start`. Returns `0` when the slot lies beyond that table's vtable,
    /// which is how a FlatBuffers writer signals a trailing field omitted
    /// because it held the default value. A stored `0` likewise means "absent".
    pub fn vtable_entry_at(&self, table_start: u32, vtable_entry: u32) -> Result<u16, DecodeError> {
        let vtable_offset = self.follow_soffset(table_start)?;
        let vtable_size = self.decode_u16(vtable_offset)?;
        // Widen to u64 so the bound comparison cannot overflow.
        if vtable_entry as u64 + 2 <= vtable_offset as u64 + vtable_size as u64 {
            self.decode_u16(vtable_entry)
        } else {
            Ok(0)
        }
    }

    /// Enter a nested table/union, bounding both recursion depth and total work.
    /// The returned [`DepthGuard`] restores the depth on drop. Call once per table
    /// body decoded.
    pub fn enter_nested(&self) -> Result<DepthGuard<'_>, DecodeError> {
        // A table is itself a unit of work.
        self.consume_budget(1)?;
        let depth = self.depth.get().saturating_add(1);
        if depth > MAX_DEPTH {
            return Err(DecodeError::ResourceLimit);
        }
        self.depth.set(depth);
        Ok(DepthGuard { depth: &self.depth })
    }

    /// Consume `n` units from the decode work budget, erroring if it is exhausted.
    /// Bounds total tables + vector elements (≈ buffer length), so a crafted buffer
    /// that aliases offsets cannot amplify into unbounded work or allocation.
    pub fn consume_budget(&self, n: u32) -> Result<(), DecodeError> {
        let remaining = self.budget.get();
        if n > remaining {
            return Err(DecodeError::ResourceLimit);
        }
        self.budget.set(remaining - n);
        Ok(())
    }
}
