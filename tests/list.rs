use femtoflatbuffers::{Decoder, Table};
use femtoflatbuffers::table::Table;

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32
}

#[cfg(feature = "alloc")]
#[derive(Table, Debug)]
struct ListTest {
    a: u32,
    b: Vec<Test>
}

#[allow(dead_code, unused_imports)]
#[path = "test_generated.rs"]
mod test;

#[cfg(feature = "alloc")]
#[test]
fn encode_test() {
    let test = ListTest{
        a: 1,
        b: vec![Test{a: 2, b: 3, c: 4}, Test{a: 5, b: 6, c: 7}],
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();
    println!("{:x?}", encoded);

    let decoded_test = flatbuffers::root::<test::test::ListTest>(&encoded).unwrap();
    println!("{:?}", decoded_test);
}

#[cfg(feature = "alloc")]
#[test]
fn decode_test() {
    let mut  builder = flatbuffers::FlatBufferBuilder::new();
    let encoded_test = {
        let sub_a = {
            let mut table_builder = test::test::TestBuilder::new(&mut builder);
            table_builder.add_a(2);
            table_builder.add_b(3);
            table_builder.add_c(4);
            table_builder.finish()
        };
        let sub_b = {
            let mut table_builder = test::test::TestBuilder::new(&mut builder);
            table_builder.add_a(5);
            table_builder.add_b(6);
            table_builder.add_c(7);
            table_builder.finish()
        };
        let fb_vec = builder.create_vector(&[sub_a, sub_b]);
        let mut table_builder = test::test::ListTestBuilder::new(&mut builder);
        table_builder.add_a(1);
        table_builder.add_b(fb_vec);
        let table = table_builder.finish();
        builder.finish(table, None);
        builder.finished_data()
    };
    println!("{:x?}", encoded_test);
    let decoded_test = ListTest::decode(&Decoder::new(&encoded_test)).unwrap();
    println!("{:?}", decoded_test);
}