use femtoflatbuffers::table::Table;
use femtoflatbuffers::{Decoder, Table, Union};

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32,
}

#[derive(Table, Debug)]
struct Test2 {
    d: u32,
    e: u32,
    f: u32,
}

// `NONE` is the conventional FlatBuffers union placeholder for the absent case;
// the derive reserves variant 0 for it.
#[allow(clippy::upper_case_acronyms)]
#[derive(Union, Debug)]
enum TestUnion {
    NONE,
    A(Test),
    B(Test2),
}

#[derive(Table, Debug)]
struct UnionTest {
    a: TestUnion,
    b: u32,
}

#[allow(warnings, clippy::all)]
#[path = "generated/test_generated.rs"]
mod test;

#[test]
fn encode_test() {
    let test = UnionTest {
        a: TestUnion::A(Test { a: 1, b: 2, c: 3 }),
        b: 2,
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();
    println!("{:x?}", encoded);

    let decoded_test = flatbuffers::root::<test::test::UnionTest>(encoded).unwrap();
    println!("{:?}", decoded_test);
}

#[test]
fn decode_test() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
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
    let decoded_test = UnionTest::decode(&Decoder::new(encoded_test)).unwrap();
    println!("{:?}", decoded_test);
}

#[test]
fn none_encode_femto_decode_flatc() {
    let test = UnionTest {
        a: TestUnion::NONE,
        b: 7,
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = flatbuffers::root::<test::test::UnionTest>(encoded).unwrap();
    assert_eq!(decoded.a_type(), test::test::TestUnion::NONE);
    assert!(decoded.a().is_none());
    assert_eq!(decoded.b(), 7);
}

#[test]
fn none_decode_from_flatc() {
    // A UnionTest with no union set at all: flatc leaves `a_type` at its NONE
    // default and omits the value. femto should decode it as TestUnion::NONE.
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = test::test::UnionTestBuilder::new(&mut builder);
        tb.add_b(7);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = UnionTest::decode(&Decoder::new(encoded)).unwrap();
    assert!(matches!(decoded.a, TestUnion::NONE));
    assert_eq!(decoded.b, 7);
}

#[test]
fn none_round_trips_through_femto() {
    let test = UnionTest {
        a: TestUnion::NONE,
        b: 99,
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = UnionTest::decode(&Decoder::new(encoded)).unwrap();
    assert!(matches!(decoded.a, TestUnion::NONE));
    assert_eq!(decoded.b, 99);
}
