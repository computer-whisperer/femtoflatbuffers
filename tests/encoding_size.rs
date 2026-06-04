//! Output-size guarantees of the encoder: default-valued fields are omitted,
//! trailing vtable entries are trimmed, and identical vtables are shared.
//! Failures here mean the output got *bigger*, not invalid — wire validity is
//! covered by the flatc cross-checks in the other test files.
#![cfg(feature = "alloc")]

use femtoflatbuffers::{Decoder, Table};

#[derive(Table, Debug, PartialEq)]
struct Test {
    a: u32,
    b: u32,
    c: u32,
}

// Wire-compatible with the `ListTest` table in tests/generated/test.fbs.
#[derive(Table, Debug, PartialEq)]
struct ListTest {
    a: u32,
    b: Vec<Test>,
}

#[allow(warnings, clippy::all)]
#[path = "generated/test_generated.rs"]
mod test_gen;

fn encode_into(buf: &mut [u8], t: &impl Table) -> usize {
    let mut enc = femtoflatbuffers::Encoder::new(buf);
    t.encode(&mut enc).unwrap();
    enc.done().len()
}

#[test]
fn all_default_table_is_minimal() {
    // Every field omitted: root uoffset (4) + table soffset (4) + trimmed
    // vtable [size=4][table_size=4] (4) = 12 bytes.
    let mut buf = [0u8; 64];
    let len = encode_into(&mut buf, &Test { a: 0, b: 0, c: 0 });
    assert_eq!(len, 12);
}

#[test]
fn identical_tables_share_one_vtable() {
    // Same-shape elements must share a vtable, so each element beyond the
    // first costs exactly its uoffset slot (4) + body (soffset 4 + 3x u32) =
    // 20 bytes -- no per-element vtable. Plus a one-time 2 bytes: the first
    // element's surviving 10-byte vtable ends 2-mod-4, so the body written
    // after it pads to 4-alignment. Without dedup the delta would be 60
    // (a 10-byte vtable per extra element).
    let element = |i: u32| Test {
        a: i,
        b: i + 1,
        c: i + 2,
    };
    let one = ListTest {
        a: 9,
        b: vec![element(1)],
    };
    let three = ListTest {
        a: 9,
        b: vec![element(1), element(11), element(21)],
    };

    let mut buf1 = [0u8; 512];
    let mut buf3 = [0u8; 512];
    let len1 = encode_into(&mut buf1, &one);
    let len3 = encode_into(&mut buf3, &three);
    assert_eq!(len3 - len1, 2 * 20 + 2, "vtables are not being shared");

    // The shared-vtable buffer still verifies and reads through flatc...
    let decoded = flatbuffers::root::<test_gen::test::ListTest>(&buf3[..len3]).unwrap();
    let b = decoded.b().unwrap();
    assert_eq!(b.len(), 3);
    assert_eq!((b.get(2).a(), b.get(2).b(), b.get(2).c()), (21, 22, 23));

    // ...and round-trips through femto.
    let back = ListTest::decode(&Decoder::new(&buf3[..len3])).unwrap();
    assert_eq!(back, three);
}

#[test]
fn trailing_default_fields_shrink_the_vtable() {
    // Only `a` set: the b/c entries are trailing zeros and must be trimmed,
    // making this strictly smaller than the same table with only `c` set
    // (whose zero entries are interior and cannot be trimmed).
    let mut buf_a = [0u8; 64];
    let mut buf_c = [0u8; 64];
    let len_a = encode_into(&mut buf_a, &Test { a: 1, b: 0, c: 0 });
    let len_c = encode_into(&mut buf_c, &Test { a: 0, b: 0, c: 1 });
    assert!(len_a < len_c, "trailing vtable zeros are not being trimmed");

    // Both still read correctly through flatc.
    let dec_a = test_gen::test::root_as_test(&buf_a[..len_a]).unwrap();
    assert_eq!((dec_a.a(), dec_a.b(), dec_a.c()), (1, 0, 0));
    let dec_c = test_gen::test::root_as_test(&buf_c[..len_c]).unwrap();
    assert_eq!((dec_c.a(), dec_c.b(), dec_c.c()), (0, 0, 1));
}

#[test]
fn deterministic_output() {
    // Padding is zeroed and rollbacks leave no stale bytes: encoding the same
    // value into a dirty buffer yields identical bytes.
    let value = ListTest {
        a: 1,
        b: vec![Test { a: 2, b: 0, c: 4 }, Test { a: 5, b: 0, c: 7 }],
    };
    let mut clean = [0u8; 512];
    let mut dirty = [0xAAu8; 512];
    let len_clean = encode_into(&mut clean, &value);
    let len_dirty = encode_into(&mut dirty, &value);
    assert_eq!(len_clean, len_dirty);
    assert_eq!(clean[..len_clean], dirty[..len_dirty]);
}
