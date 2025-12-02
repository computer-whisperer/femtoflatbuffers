use femtoflatbuffers::{Decoder, Table};
use femtoflatbuffers::table::Table;

#[derive( Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32
}
impl femtoflatbuffers::table::Table for Test {
    fn encode(&self, encoder: &mut femtoflatbuffers::Encoder) -> Result<(), femtoflatbuffers::EncodeError> {
        encoder.encode_u32(4)?;
        {
            let start = encoder.encode_i32(0)?;
            let a_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.a, encoder, start)?;
            let b_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.b, encoder, start)?;
            let c_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.c, encoder, start)?;
            let table_end = encoder.used_bytes();
            let vtable_start = encoder.encode_u16(0)?;
            encoder.encode_i32_at(start, -((vtable_start - start) as i32))?;
            encoder.encode_u16((table_end - start) as u16)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.a, encoder, vtable_start, &a_working_value)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.b, encoder, vtable_start, &b_working_value)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.c, encoder, vtable_start, &c_working_value)?;
            encoder.encode_u16_at(vtable_start, (encoder.used_bytes() - vtable_start) as u16)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.a, encoder, &a_working_value)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.b, encoder, &b_working_value)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.c, encoder, &c_working_value)?;
            Ok(start)
        }?;
        Ok(())
    }
    fn decode(decoder: &femtoflatbuffers::Decoder) -> Result<Self, femtoflatbuffers::DecodeError> {
        let root_offset = decoder.decode_u32(0)?;
        let vtable_offset = ((root_offset as i32) - decoder.decode_i32(root_offset)?) as u32;
        let vtable_size = decoder.decode_u16(vtable_offset)?;
        let table_size = decoder.decode_u16(vtable_offset + 2)?;
        let a_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 4u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let b_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 6u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let c_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 8u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let res = Test {
            a: femtoflatbuffers::ComponentDecode::value_decode(&decoder, a_offset_blah.map(|x| x as u32 + root_offset))?,
            b: femtoflatbuffers::ComponentDecode::value_decode(&decoder, b_offset_blah.map(|x| x as u32 + root_offset))?,
            c: femtoflatbuffers::ComponentDecode::value_decode(&decoder, c_offset_blah.map(|x| x as u32 + root_offset))?,
        };
        Ok(res)
    }
}
impl femtoflatbuffers::ComponentEncode for Test {
    type WorkingValue = (u32, u32);
    fn value_encode(&self, encoder: &mut femtoflatbuffers::Encoder, table_start: u32) -> Result<Self::WorkingValue, femtoflatbuffers::EncodeError> {
        let value_offset = encoder.encode_i32(0)?;
        Ok((table_start, value_offset))
    }
    fn vtable_encode(&self, encoder: &mut femtoflatbuffers::Encoder, _vtable_start: u32, working_value: &Self::WorkingValue) -> Result<(), femtoflatbufferes::EncodeError> {
        encoder.encode_u16((working_value.1 - working_value.0) as u16)?;
        Ok(())
    }
    fn post_encode(&self, encoder: &mut femtoflatbuffers::Encoder, working_value: &Self::WorkingValue) -> Result<(), femtoflatbuffers::EncodeError> {
        match {
            let start = encoder.encode_i32(0)?;
            let a_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.a, encoder, start)?;
            let b_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.b, encoder, start)?;
            let c_working_value = femtoflatbuffers::ComponentEncode::value_encode(&self.c, encoder, start)?;
            let table_end = encoder.used_bytes();
            let vtable_start = encoder.encode_u16(0)?;
            encoder.encode_i32_at(start, -((vtable_start - start) as i32))?;
            encoder.encode_u16((table_end - start) as u16)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.a, encoder, vtable_start, &a_working_value)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.b, encoder, vtable_start, &b_working_value)?;
            femtoflatbuffers::ComponentEncode::vtable_encode(&self.c, encoder, vtable_start, &c_working_value)?;
            encoder.encode_u16_at(vtable_start, (encoder.used_bytes() - vtable_start) as u16)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.a, encoder, &a_working_value)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.b, encoder, &b_working_value)?;
            femtoflatbuffers::ComponentEncode::post_encode(&self.c, encoder, &c_working_value)?;
            Ok(start)
        } {
            Ok(global_table_offset) => {
                let global_field_offset = working_value.1;
                encoder.encode_i32_at(global_field_offset, (global_table_offset - global_field_offset) as i32)?;
                Ok(())
            }
            Err(err) => Err(err)
        }
    }
}
impl femtoflatbuffers::ComponentDecode for Test {
    type WorkingValue = (u32, u16);
    type VectorWorkingValue = Self::WorkingValue;
    fn vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::WorkingValue, u32), DecodeError> {
        let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
        Ok(((table_start, vtable_entry_value), vtable_entry + 2))
    }
    fn value_decode(decoder: &femtoflatbuffers::Decoder, working_value: &Self::WorkingValue) -> Result<Self, femtoflatbuffers::DecodeError> {
        let root_offset = (working_value.0 as i32 + decoder.decode_i32(working_value.0 + working_value.1 as u32)?) as u32;
        let vtable_offset = ((root_offset as i32) - decoder.decode_i32(root_offset)?) as u32;
        let vtable_size = decoder.decode_u16(vtable_offset)?;
        let table_size = decoder.decode_u16(vtable_offset + 2)?;
        let a_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 4u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let b_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 6u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let c_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 8u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let res = Test {
            a: femtoflatbuffers::ComponentDecode::value_decode(&decoder, a_offset_blah.map(|x| x as u32 + root_offset))?,
            b: femtoflatbuffers::ComponentDecode::value_decode(&decoder, b_offset_blah.map(|x| x as u32 + root_offset))?,
            c: femtoflatbuffers::ComponentDecode::value_decode(&decoder, c_offset_blah.map(|x| x as u32 + root_offset))?,
        };
        Ok(res)
    }
    fn vector_vtable_decode(decoder: &Decoder, table_start: u32, vtable_entry: u32) -> Result<(Self::VectorWorkingValue, u32), DecodeError> {
        let vtable_entry_value = decoder.decode_u16(vtable_entry)?;
        Ok(((table_start, vtable_entry_value), vtable_entry + 2))
    }
    fn vector_len_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue) -> Result<usize, DecodeError> {
        let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32) as u32;
        Ok(decoder.decode_u32(vector_offset)? as usize)
    }
    fn vector_value_decode(decoder: &Decoder, working_value: &Self::VectorWorkingValue, idx: usize) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let vector_offset = (decoder.decode_i32(working_value.0 + working_value.1 as u32)? + working_value.0 as i32) as u32;
        let vector_entry_offset = (vector_offset + 4) + (idx * 4) as u32;
        let root_offset = (vector_entry_offset as i32 + decoder.decode_i32(vector_entry_offset)) as u32;
        let vtable_offset = ((root_offset as i32) - decoder.decode_i32(root_offset)?) as u32;
        let vtable_size = decoder.decode_u16(vtable_offset)?;
        let table_size = decoder.decode_u16(vtable_offset + 2)?;
        let a_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 4u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let b_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 6u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let c_offset_blah = {
            let val = decoder.decode_u16(vtable_offset + 8u32)?;
            if val == 0 { None } else { Some(val) }
        };
        let res = Test {
            a: femtoflatbuffers::ComponentDecode::value_decode(&decoder, a_offset_blah.map(|x| x as u32 + root_offset))?,
            b: femtoflatbuffers::ComponentDecode::value_decode(&decoder, b_offset_blah.map(|x| x as u32 + root_offset))?,
            c: femtoflatbuffers::ComponentDecode::value_decode(&decoder, c_offset_blah.map(|x| x as u32 + root_offset))?,
        };
        Ok(res) } }

#[allow(dead_code, unused_imports)]
#[path = "test_generated.rs"]
mod test;

#[test]
fn encode_test() {
    let test = Test{
        a: 1,
        b: 2,
        c: 3
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();
    println!("{:x?}", encoded);

    let decoded_test = test::test::root_as_test(&encoded).unwrap();
    println!("{:?}", decoded_test);
}

#[test]
fn decode_test() {
    let mut  builder = flatbuffers::FlatBufferBuilder::new();
    let encoded_test = {
        let mut table_builder = test::test::TestBuilder::new(&mut builder);
        table_builder.add_a(1);
        table_builder.add_b(2);
        table_builder.add_c(3);
        let table = table_builder.finish();
        builder.finish(table, None);
        builder.finished_data()
    };
    println!("{:x?}", encoded_test);
    let decoded_test = Test::decode(&Decoder::new(&encoded_test)).unwrap();
    println!("{:?}", decoded_test);
}