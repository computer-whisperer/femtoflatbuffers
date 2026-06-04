//! Vectors of scalars, cross-checked against the `flatbuffers` crate. The
//! 8-byte element types ([ulong]/[double]) are the regression surface for
//! vector-element alignment: the length prefix must be positioned so elements
//! land at `start + 4` already 8-aligned, with no padding gap after the prefix.
//! `flatbuffers::root` runs the official verifier, which checks exactly that.
#![cfg(feature = "alloc")]

use femtoflatbuffers::{Decoder, Table};

// Wire-compatible with tests/generated/vec_scalars_test.fbs.
#[derive(Table, Debug, PartialEq)]
struct VecScalarsTest {
    a: u32,
    b: Vec<u64>,
    c: Vec<u8>,
    d: Vec<f64>,
}

#[allow(warnings, clippy::all)]
#[path = "generated/vec_scalars_test_generated.rs"]
mod vec_scalars_gen;

fn sample() -> VecScalarsTest {
    VecScalarsTest {
        a: 1,
        b: vec![0x1111_2222_3333_4444, 0x5555_6666_7777_8888],
        c: vec![9, 8, 7],
        d: vec![1.5, -2.25],
    }
}

#[test]
fn encode_femto_decode_flatc() {
    let test = sample();

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    // `root` runs the official verifier, which rejects misaligned vectors.
    let decoded =
        flatbuffers::root::<vec_scalars_gen::vec_scalars_test::VecScalarsTest>(encoded).unwrap();
    assert_eq!(decoded.a(), 1);
    assert_eq!(
        decoded.b().unwrap().iter().collect::<Vec<_>>(),
        vec![0x1111_2222_3333_4444, 0x5555_6666_7777_8888]
    );
    assert_eq!(decoded.c().unwrap().bytes(), &[9, 8, 7]);
    assert_eq!(
        decoded.d().unwrap().iter().collect::<Vec<_>>(),
        vec![1.5, -2.25]
    );
}

#[test]
fn encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let b = builder.create_vector::<u64>(&[0x1111_2222_3333_4444, 0x5555_6666_7777_8888]);
        let c = builder.create_vector::<u8>(&[9, 8, 7]);
        let d = builder.create_vector::<f64>(&[1.5, -2.25]);
        let mut tb = vec_scalars_gen::vec_scalars_test::VecScalarsTestBuilder::new(&mut builder);
        tb.add_a(1);
        tb.add_b(b);
        tb.add_c(c);
        tb.add_d(d);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = VecScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, sample());
}

#[test]
fn femto_round_trip() {
    let test = sample();

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = VecScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);
}

#[test]
fn vector_elements_with_zeros_round_trip() {
    // Default omission applies to table fields, never vector elements: zeros
    // inside a vector must keep their slots.
    let test = VecScalarsTest {
        a: 0,
        b: vec![0, 1, 0],
        c: vec![0, 0, 0],
        d: vec![0.0, -2.25],
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = VecScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);

    // The official verifier + reader agree on the zero-bearing vectors.
    let via_flatc =
        flatbuffers::root::<vec_scalars_gen::vec_scalars_test::VecScalarsTest>(encoded).unwrap();
    assert_eq!(
        via_flatc.b().unwrap().iter().collect::<Vec<_>>(),
        vec![0, 1, 0]
    );
    assert_eq!(via_flatc.c().unwrap().bytes(), &[0, 0, 0]);
}

#[test]
fn femto_round_trip_empty_vectors() {
    let test = VecScalarsTest {
        a: 5,
        b: vec![],
        c: vec![],
        d: vec![],
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = VecScalarsTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);
}
