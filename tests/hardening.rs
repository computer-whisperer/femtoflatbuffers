//! Decoder robustness against malformed/adversarial input. The contract: for any
//! byte slice, `decode` returns `Ok`/`Err` but never panics, never overflows the
//! stack, and never makes an unbounded allocation. These tests feed truncated and
//! mutated buffers and rely on the fact that a panic (or OOM/stack overflow) in a
//! test aborts it -> the test failing.
#![cfg(feature = "alloc")]

use femtoflatbuffers::{Decoder, Encoder, Table};

#[derive(Table, Debug)]
struct Inner {
    x: u32,
    y: i16,
}

// Exercises every decode path that does offset arithmetic / recursion / allocation:
// a scalar, an optional nested table, a vector of tables, and a string.
#[derive(Table, Debug)]
struct Outer {
    a: u32,
    b: Option<Inner>,
    c: Vec<Inner>,
    s: String,
}

fn good_buffer() -> Vec<u8> {
    let outer = Outer {
        a: 0xDEAD_BEEF,
        b: Some(Inner { x: 1, y: -2 }),
        c: vec![Inner { x: 3, y: 4 }, Inner { x: 5, y: 6 }],
        s: "hardening".to_string(),
    };
    let mut buf = [0u8; 1024];
    let mut enc = Encoder::new(&mut buf);
    outer.encode(&mut enc).unwrap();
    enc.done().to_vec()
}

#[test]
fn good_buffer_round_trips() {
    let buf = good_buffer();
    let out = Outer::decode(&Decoder::new(&buf)).unwrap();
    assert_eq!(out.a, 0xDEAD_BEEF);
    assert_eq!(out.b.unwrap().y, -2);
    assert_eq!(out.c.len(), 2);
    assert_eq!(out.s, "hardening");
}

#[test]
fn out_of_range_root_errors() {
    // Too short to even hold the root offset.
    assert!(Outer::decode(&Decoder::new(&[])).is_err());
    assert!(Outer::decode(&Decoder::new(&[0])).is_err());
    // Root offset points far outside the buffer.
    assert!(Outer::decode(&Decoder::new(&[0xFF, 0xFF, 0xFF, 0xFF])).is_err());
    // Note: [0,0,0,0] is *not* an error -- it is a degenerate but well-formed
    // buffer (root -> offset 0, zero-size vtable) that decodes to all defaults.
    assert!(Outer::decode(&Decoder::new(&[0, 0, 0, 0])).is_ok());
}

#[test]
fn every_truncation_is_safe() {
    let buf = good_buffer();
    for len in 0..=buf.len() {
        // The only assertion is that this returns rather than panicking.
        let _ = Outer::decode(&Decoder::new(&buf[..len]));
    }
}

#[test]
fn single_byte_mutations_are_safe() {
    let base = good_buffer();
    for i in 0..base.len() {
        for v in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
            let mut buf = base.clone();
            buf[i] = v;
            let _ = Outer::decode(&Decoder::new(&buf));
        }
    }
}

#[test]
fn word_mutations_are_safe() {
    // Slide an all-0xFF 4-byte window across the buffer. This reliably produces
    // huge claimed vector/string lengths and wild (negative) offsets at every
    // position -- the cases that would overflow, over-allocate, or recurse.
    let base = good_buffer();
    let mut i = 0;
    while i + 4 <= base.len() {
        let mut buf = base.clone();
        for b in &mut buf[i..i + 4] {
            *b = 0xFF;
        }
        let _ = Outer::decode(&Decoder::new(&buf));
        i += 1;
    }
}
