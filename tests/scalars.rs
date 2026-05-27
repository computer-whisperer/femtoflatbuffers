//! Runtime tests for the f32/f64/bool/i8 primitive types, cross-checked for
//! wire compatibility against the official `flatbuffers` crate in both
//! directions.

use femtoflatbuffers::table::Table;
use femtoflatbuffers::{Decoder, Table};

// Wire-compatible with the `ScalarsTest` table in tests/scalars_test.fbs
// (f: float, d: double, b: bool, i: byte).
#[derive(Table, Debug, PartialEq)]
struct ScalarsTest {
    f: f32,
    d: f64,
    b: bool,
    i: i8,
}

#[allow(warnings, clippy::all)]
#[path = "generated/scalars_test_generated.rs"]
mod scalars_gen;

// All fields use non-default values so flatc actually stores them; femto does
// not yet synthesize omitted-default scalars on decode.
const SAMPLE: ScalarsTest = ScalarsTest {
    f: 1.5,
    d: -2.25,
    b: true,
    i: -3,
};

#[test]
fn encode_femto_decode_flatc() {
    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    SAMPLE.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = scalars_gen::scalars_test::root_as_scalars_test(encoded).unwrap();
    assert_eq!(decoded.f(), 1.5);
    assert_eq!(decoded.d(), -2.25);
    assert!(decoded.b());
    assert_eq!(decoded.i(), -3);
}

#[test]
fn encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = scalars_gen::scalars_test::ScalarsTestBuilder::new(&mut builder);
        tb.add_f(1.5);
        tb.add_d(-2.25);
        tb.add_b(true);
        tb.add_i(-3);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = ScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, SAMPLE);
}

#[test]
fn decode_flatc_omitting_trailing_defaults() {
    // flatc writes only `f`; d/b/i hold their defaults and are dropped from the
    // tail of the vtable entirely, so the vtable is shorter than the field count.
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = scalars_gen::scalars_test::ScalarsTestBuilder::new(&mut builder);
        tb.add_f(1.5);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = ScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(
        decoded,
        ScalarsTest {
            f: 1.5,
            d: 0.0,
            b: false,
            i: 0
        }
    );
}

#[test]
fn decode_flatc_omitting_interior_defaults() {
    // flatc writes `f` and `i` but not `d`/`b`; since `i` follows them, the
    // omitted fields appear as zero vtable entries rather than truncation.
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let mut tb = scalars_gen::scalars_test::ScalarsTestBuilder::new(&mut builder);
        tb.add_f(1.5);
        tb.add_i(-3);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = ScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(
        decoded,
        ScalarsTest {
            f: 1.5,
            d: 0.0,
            b: false,
            i: -3
        }
    );
}

#[test]
fn femto_round_trip_with_defaults() {
    // femto always writes every non-Option field, so a default-valued field
    // (b: false, i: 0) round-trips through femto's own encode/decode.
    let original = ScalarsTest {
        f: 0.0,
        d: 3.0,
        b: false,
        i: 0,
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    original.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = ScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, original);
}
