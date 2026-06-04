//! Runtime tests for the `heapless` feature: `heapless::Vec<T, N>` and
//! `heapless::String<N>` fields, cross-checked for wire compatibility against
//! the official `flatbuffers` crate in both directions.
#![cfg(feature = "heapless")]

use femtoflatbuffers::table::Table;
use femtoflatbuffers::{Decoder, Table};

#[derive(Table, Debug)]
struct Test {
    a: u32,
    b: u32,
    c: u32,
}

// Wire-compatible with the `ListTest` table in tests/test.fbs (a: int, b: [Test]),
// but backed by a fixed-capacity heapless::Vec instead of alloc::Vec.
#[derive(Table, Debug)]
struct ListTest {
    a: u32,
    b: heapless::Vec<Test, 8>,
}

// Wire-compatible with the `StringTest` table in tests/string_test.fbs
// (a: uint, s: string).
#[derive(Table, Debug)]
struct StringTest {
    a: u32,
    s: heapless::String<32>,
}

#[allow(warnings, clippy::all)]
#[path = "generated/test_generated.rs"]
mod test;

#[allow(warnings, clippy::all)]
#[path = "generated/string_test_generated.rs"]
mod string_gen;

fn vec_of<T, const N: usize>(items: impl IntoIterator<Item = T>) -> heapless::Vec<T, N> {
    let mut v = heapless::Vec::new();
    for item in items {
        v.push(item)
            .ok()
            .expect("heapless::Vec capacity exceeded in test setup");
    }
    v
}

fn string_of<const N: usize>(s: &str) -> heapless::String<N> {
    let mut out = heapless::String::new();
    out.push_str(s)
        .expect("heapless::String capacity exceeded in test setup");
    out
}

// Wire-compatible with the [ulong] field of vec_scalars_test.fbs; exercises
// vector-element alignment (8-byte elements must land at `start + 4`).
#[derive(Table, Debug, PartialEq)]
struct VecU64Test {
    a: u32,
    b: heapless::Vec<u64, 8>,
}

// --- heapless::Vec ---------------------------------------------------------

#[test]
fn vec_u64_round_trips_aligned() {
    let test = VecU64Test {
        a: 1,
        b: vec_of([0x1111_2222_3333_4444u64, 0x5555_6666_7777_8888]),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = VecU64Test::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded, test);
}

#[test]
fn vec_encode_femto_decode_flatc() {
    let test = ListTest {
        a: 1,
        b: vec_of([Test { a: 2, b: 3, c: 4 }, Test { a: 5, b: 6, c: 7 }]),
    };

    let mut buffer = [0u8; 1024];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = flatbuffers::root::<test::test::ListTest>(encoded).unwrap();
    assert_eq!(decoded.a(), 1);
    let b = decoded.b().unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!((b.get(0).a(), b.get(0).b(), b.get(0).c()), (2, 3, 4));
    assert_eq!((b.get(1).a(), b.get(1).b(), b.get(1).c()), (5, 6, 7));
}

#[test]
fn vec_encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let sub_a = {
            let mut tb = test::test::TestBuilder::new(&mut builder);
            tb.add_a(2);
            tb.add_b(3);
            tb.add_c(4);
            tb.finish()
        };
        let sub_b = {
            let mut tb = test::test::TestBuilder::new(&mut builder);
            tb.add_a(5);
            tb.add_b(6);
            tb.add_c(7);
            tb.finish()
        };
        let fb_vec = builder.create_vector(&[sub_a, sub_b]);
        let mut tb = test::test::ListTestBuilder::new(&mut builder);
        tb.add_a(1);
        tb.add_b(fb_vec);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = ListTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded.a, 1);
    assert_eq!(decoded.b.len(), 2);
    assert_eq!((decoded.b[0].a, decoded.b[0].b, decoded.b[0].c), (2, 3, 4));
    assert_eq!((decoded.b[1].a, decoded.b[1].b, decoded.b[1].c), (5, 6, 7));
}

#[test]
fn vec_empty_round_trips() {
    let test = ListTest {
        a: 9,
        b: vec_of::<Test, 8>([]),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = ListTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded.a, 9);
    assert!(decoded.b.is_empty());
}

// --- heapless::String ------------------------------------------------------

#[test]
fn string_encode_femto_decode_flatc() {
    let test = StringTest {
        a: 42,
        s: string_of("hello"),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = string_gen::string_test::root_as_string_test(encoded).unwrap();
    assert_eq!(decoded.a(), 42);
    assert_eq!(decoded.s(), Some("hello"));
}

#[test]
fn string_encode_flatc_decode_femto() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let encoded = {
        let s = builder.create_string("hello");
        let mut tb = string_gen::string_test::StringTestBuilder::new(&mut builder);
        tb.add_a(42);
        tb.add_s(s);
        let table = tb.finish();
        builder.finish(table, None);
        builder.finished_data()
    };

    let decoded = StringTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded.a, 42);
    assert_eq!(decoded.s.as_str(), "hello");
}

#[test]
fn string_empty_round_trips() {
    let test = StringTest {
        a: 7,
        s: string_of(""),
    };

    let mut buffer = [0u8; 256];
    let mut encoder = femtoflatbuffers::Encoder::new(&mut buffer);
    test.encode(&mut encoder).unwrap();
    let encoded = encoder.done();

    let decoded = StringTest::decode(&Decoder::new(encoded)).unwrap();
    assert_eq!(decoded.a, 7);
    assert_eq!(decoded.s.as_str(), "");
}
