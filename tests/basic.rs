use femtoflatbuffers::{Decoder, Table};
use femtoflatbuffers::table::Table;

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32
}

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