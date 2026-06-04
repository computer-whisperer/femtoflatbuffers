use femtoflatbuffers::{Decoder, Table};

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32,
}

#[allow(warnings, clippy::all)]
#[path = "generated/test_generated.rs"]
mod test;

#[test]
fn encode_test() {
    let test = Test { a: 1, b: 2, c: 3 };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();
    println!("{:x?}", encoded);

    let decoded_test = test::test::root_as_test(encoded).unwrap();
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
        let table = table_builder.finish();
        builder.finish(table, None);
        builder.finished_data()
    };
    println!("{:x?}", encoded_test);
    let decoded_test = Test::decode(&Decoder::new(encoded_test)).unwrap();
    println!("{:?}", decoded_test);
}

// Wire-compatible with `Test`, but with optional fields: a `None` encodes as an
// absent vtable entry, which a FlatBuffers reader sees as the field's default.
#[derive(Table, Debug, PartialEq)]
struct OptionalTest {
    a: Option<u32>,
    b: u32,
    c: Option<u32>,
}

#[test]
fn option_encode_femto_decode_flatc() {
    let test = OptionalTest {
        a: None,
        b: 2,
        c: Some(3),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = test::test::root_as_test(encoded).unwrap();
    assert_eq!(decoded.a(), 0); // absent -> schema default
    assert_eq!(decoded.b(), 2);
    assert_eq!(decoded.c(), 3);
}

#[test]
fn option_encode_flatc_decode_femto() {
    // flatc omits default-valued fields, so only `b` gets a vtable entry; the
    // omitted `a`/`c` must decode as None.
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = test::test::TestBuilder::new(&mut builder);
        tb.add_b(2);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = OptionalTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(
        decoded,
        OptionalTest {
            a: None,
            b: 2,
            c: None
        }
    );
}
