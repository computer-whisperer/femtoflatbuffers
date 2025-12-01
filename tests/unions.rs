use femtoflatbuffers::{Decoder, Table, Union};
use femtoflatbuffers::table::Table;

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32
}

#[derive(Table, Debug)]
struct Test2 {
    d: u32,
    e: u32,
    f: u32
}

#[derive(Union, Debug)]
enum TestUnion {
    NONE,
    A(Test),
    B(Test2)
}

#[derive(Table, Debug)]
struct UnionTest {
    a: TestUnion,
    b: u32
}

#[allow(dead_code, unused_imports)]
#[path = "test_generated.rs"]
mod test;

#[test]
fn encode_test() {
    let test = UnionTest{
        a: TestUnion::A(Test{
            a: 1,
            b: 2,
            c: 3
        }),
        b: 2,
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();
    println!("{:x?}", encoded);

    let decoded_test = flatbuffers::root::<test::test::UnionTest>(&encoded).unwrap();
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
        let test = table_builder.finish().as_union_value();
        let mut table_builder = test::test::UnionTestBuilder::new(&mut builder);
        table_builder.add_a_type(test::test::TestUnion::A);
        table_builder.add_a(test);
        table_builder.add_b(3);
        let table = table_builder.finish();
        builder.finish(table, None);
        builder.finished_data()
    };
    println!("{:x?}", encoded_test);
    let decoded_test = UnionTest::decode(&Decoder::new(&encoded_test)).unwrap();
    println!("{:?}", decoded_test);
}