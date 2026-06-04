//! 64-bit table fields (u64/i64), cross-checked against the `flatbuffers`
//! crate in both directions. The leading u8 field forces the 8-byte fields to
//! self-align inside the table; `flatbuffers::root` runs the official
//! verifier, which checks that alignment.

use femtoflatbuffers::{Decoder, Table};

// Wire-compatible with tests/generated/scalars64_test.fbs.
#[derive(Table, Debug, PartialEq)]
struct Scalars64Test {
    a: u8,
    b: u64,
    c: i64,
    d: u32,
}

#[allow(warnings, clippy::all)]
#[path = "generated/scalars64_test_generated.rs"]
mod scalars64_gen;

const SAMPLE: Scalars64Test = Scalars64Test {
    a: 7,
    b: 0x1111_2222_3333_4444,
    c: -0x0555_6666_7777_8888,
    d: 9,
};

#[test]
fn encode_femto_decode_flatc() {
    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    SAMPLE.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded =
        flatbuffers::root::<scalars64_gen::scalars_64_test::Scalars64Test>(encoded).unwrap();
    assert_eq!(decoded.a(), 7);
    assert_eq!(decoded.b(), 0x1111_2222_3333_4444);
    assert_eq!(decoded.c(), -0x0555_6666_7777_8888);
    assert_eq!(decoded.d(), 9);
}

#[test]
fn encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = scalars64_gen::scalars_64_test::Scalars64TestBuilder::new(&mut builder);
        tb.add_a(7);
        tb.add_b(0x1111_2222_3333_4444);
        tb.add_c(-0x0555_6666_7777_8888);
        tb.add_d(9);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = Scalars64Test::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, SAMPLE);
}

#[test]
fn femto_round_trip() {
    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    SAMPLE.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = Scalars64Test::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, SAMPLE);
}
