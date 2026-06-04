//! Vectors of strings (`Vec<String>`), cross-checked against the
//! `flatbuffers` crate in both directions. `flatbuffers::root` runs the
//! official verifier, which checks every element uoffset and string body.
#![cfg(feature = "alloc")]

use femtoflatbuffers::{Decoder, Table};

// Wire-compatible with tests/generated/string_vec_test.fbs.
#[derive(Table, Debug, PartialEq)]
struct StringVecTest {
    a: u32,
    b: Vec<String>,
}

#[allow(warnings, clippy::all)]
#[path = "generated/string_vec_test_generated.rs"]
mod string_vec_gen;

fn sample() -> StringVecTest {
    StringVecTest {
        a: 1,
        // The empty element matters: as a table field an empty string is
        // omitted, but as a vector element it must still occupy its slot.
        b: vec!["hello".to_string(), String::new(), "世界".to_string()],
    }
}

#[test]
fn encode_femto_decode_flatc() {
    let test = sample();

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded =
        flatbuffers::root::<string_vec_gen::string_vec_test::StringVecTest>(encoded).unwrap();
    assert_eq!(decoded.a(), 1);
    let b = decoded.b().unwrap();
    assert_eq!(b.len(), 3);
    assert_eq!(b.get(0), "hello");
    assert_eq!(b.get(1), "");
    assert_eq!(b.get(2), "世界");
}

#[test]
fn encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let strings: Vec<_> = ["hello", "", "世界"]
            .iter()
            .map(|s| builder.create_string(s))
            .collect();
        let b = builder.create_vector(&strings);
        let mut tb = string_vec_gen::string_vec_test::StringVecTestBuilder::new(&mut builder);
        tb.add_a(1);
        tb.add_b(b);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = StringVecTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, sample());
}

#[test]
fn femto_round_trip() {
    let test = sample();

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = StringVecTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);
}

#[test]
fn femto_round_trip_empty_vector() {
    let test = StringVecTest { a: 5, b: vec![] };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = StringVecTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);
}
