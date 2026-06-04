//! Runtime tests for `alloc::String` fields, cross-checked for wire
//! compatibility against the official `flatbuffers` crate in both directions.
#![cfg(feature = "alloc")]

use femtoflatbuffers::{Decoder, Table};

// Wire-compatible with the `StringTest` table in tests/string_test.fbs
// (a: uint, s: string).
#[derive(Table, Debug, PartialEq)]
struct StringTest {
    a: u32,
    s: String,
}

#[allow(warnings, clippy::all)]
#[path = "generated/string_test_generated.rs"]
mod string_gen;

#[test]
fn encode_femto_decode_flatc() {
    let test = StringTest {
        a: 42,
        s: "héllo, 世界".to_string(),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = string_gen::string_test::root_as_string_test(encoded).unwrap();
    assert_eq!(decoded.a(), 42);
    assert_eq!(decoded.s(), Some("héllo, 世界"));
}

#[test]
fn encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let s = builder.create_string("héllo, 世界");
        let mut tb = string_gen::string_test::StringTestBuilder::new(&mut builder);
        tb.add_a(42);
        tb.add_s(s);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = StringTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(
        decoded,
        StringTest {
            a: 42,
            s: "héllo, 世界".to_string()
        }
    );
}

#[test]
fn empty_string_round_trips() {
    let test = StringTest {
        a: 7,
        s: String::new(),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = StringTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(
        decoded,
        StringTest {
            a: 7,
            s: String::new()
        }
    );
}
