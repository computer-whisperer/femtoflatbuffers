# femtoflatbuffers

A tiny, `#![no_std]`, allocation-optional [FlatBuffers](https://flatbuffers.dev/)
encoder/decoder for Rust, driven entirely by derive macros instead of a schema
compiler.

Where the official [`flatbuffers`](https://crates.io/crates/flatbuffers) crate
generates Rust code from a `.fbs` schema via the `flatc` compiler, in
`femtoflatbuffers` **the Rust type _is_ the schema**: you annotate plain structs
and enums with `#[derive(Table)]` / `#[derive(Union)]` and get wire-compatible
FlatBuffers serialization with no build step, no codegen, and no heap (unless you
ask for one).

```rust
use femtoflatbuffers::{Table, Encoder, Decoder};

#[derive(Table, Debug)]
struct Monster {
    hp: u16,
    mana: u16,
    level: u32,
}

let m = Monster { hp: 100, mana: 50, level: 7 };

// Encode into a caller-owned, fixed-size buffer — no allocation.
let mut buf = [0u8; 256];
let mut enc = Encoder::new(&mut buf);
m.encode(&mut enc).unwrap();
let bytes: &[u8] = enc.done();

// Decode from any &[u8].
let back = Monster::decode(&Decoder::new(bytes)).unwrap();
assert_eq!(back.level, 7);
```

The output of `encode` is byte-compatible with a buffer produced by the official
`flatc`-generated code for the equivalent schema, and vice versa. The test suite
cross-checks both directions against the `flatbuffers` crate (see
[`tests/`](tests/) and [`tests/test.fbs`](tests/test.fbs)).

## Why

FlatBuffers is attractive on microcontrollers — zero-copy reads, compact wire
format, forward/backward-compatible vtables. But the official Rust pipeline
assumes `std`, an allocator, and a `flatc` build step. `femtoflatbuffers` targets
the "femto" end: firmware that wants to speak the FlatBuffers wire format to a
larger host, while staying `no_std` and (optionally) heap-free, and while keeping
the schema as ordinary Rust types that the rest of the firmware already uses.

## Design

### Buffers, not builders

Both directions work against a flat byte slice the caller owns:

- `Encoder::new(&mut [u8])` is a forward bump-writer. It tracks `used_bytes` and
  appends little-endian values, padding each to its natural alignment. `done()`
  returns the filled prefix. If the buffer fills up, every write returns
  `EncodeError::OutOfSpace` rather than panicking or allocating.
- `Decoder::new(&[u8])` does bounds-checked little-endian reads at absolute
  offsets.

Notably, this builds the buffer **front-to-back**, whereas the canonical
FlatBuffers builder writes back-to-front. Front-to-back keeps the encoder simple
on constrained targets (a plain cursor) at the cost of needing to revisit and
patch forward offsets after the fact — which is what the three-phase protocol
below is for.

### The three-phase component protocol

Every serializable field type implements `ComponentEncode` / `ComponentDecode`
(see [`src/components.rs`](src/components.rs)). The derive macro
([`derive/src/lib.rs`](derive/src/lib.rs)) emits these impls for tables and
unions; primitives, `Option<T>`, `Vec<T>` (alloc), and `heapless` collections get
hand-written impls.

Encoding a table walks its fields three times so that fixed-size and
variable-size data end up in the right regions of the buffer:

1. **`value_encode`** — lay out the field's slot in the fixed table region.
   Primitives write their value inline here; tables/vectors/strings write a 4-byte
   placeholder that will later hold a forward offset. Returns a "working value"
   carrying the offsets needed by later phases.
2. **`vtable_encode`** — write this field's `u16` entry in the vtable (the field's
   offset within the table, or `0` if absent).
3. **`post_encode`** — append variable-length payloads (nested table bodies,
   vector contents, string bytes) after the table+vtable, then back-patch the
   placeholder offset from phase 1 to point at them.

The generated table layout is `[table body][vtable][nested payloads...]`, with the
table's `soffset` pointing forward to its vtable. The root is a `u32` uoffset at
byte 0 pointing at the root table.

Decoding mirrors this: `vtable_decode` locates a field's vtable entry,
`value_decode` follows it to read the value (or chases the forward offset for
nested/variable data), and the vector variants add length-prefixed iteration.

## Feature flags

| Feature    | Default | Enables                                                         |
|------------|---------|----------------------------------------------------------------|
| (none)     | ✓       | Primitives, `Option<T>`, nested `#[derive(Table)]`, `#[derive(Union)]` |
| `alloc`    |         | `Vec<T>` fields (requires a global allocator)                  |
| `heapless` |         | `heapless::Vec<T, N>` and `heapless::String<N>` fields, fully heap-free |

The crate is `#![no_std]` unconditionally; `alloc` only pulls in `extern crate alloc`.

## Supported types

- Integers: `u8`, `u16`/`i16`, `u32`/`i32`, `u64`/`i64`. (No `i8` impl — see
  Limitations.)
- `Option<T>` — maps to FlatBuffers' "field absent" (vtable entry `0`).
- Nested tables via `#[derive(Table)]`.
- Unions via `#[derive(Union)]` on an enum whose first variant is the `NONE`
  marker and whose remaining variants each wrap a single table type.
- Vectors: `Vec<T>` (`alloc`) or `heapless::Vec<T, N>` (`heapless`).
- Strings: `heapless::String<N>` (`heapless` only).

## Known issues & limitations

This crate was written pre-documentation and has rough edges. As of this review:

- **`heapless` support is unverified at runtime.** It now compiles, but there are
  no tests exercising `heapless::Vec`/`String` encode/decode, so correctness on
  that path is untested.
- **No string support without `heapless`.** There is `Vec<T>` for `alloc` but no
  `String`/`&str` impl for `alloc` or core. Strings are only available via
  `heapless::String`.
- **No floating-point support.** `f32`/`f64` are not implemented as primitives.
- **No `bool` / `i8` primitive impls.** Only the integer types listed above
  (`i8` is absent even though `u8` is present).
- **Vtables are not deduplicated.** The canonical format reuses one vtable across
  identical tables; here every table emits its own. Output is still valid, just
  larger.
- **No default-value omission.** Non-`Option` scalar fields are always written,
  even when equal to the schema default. The canonical writer omits defaults.
- **Decoder trusts its input.** Bounds are checked per-read, but offset arithmetic
  uses `as` casts and unchecked `+`, so adversarial/corrupt buffers can produce
  wrong results rather than clean errors. Treat input as trusted.
- **Unions:** decoding the `NONE` case returns `InvalidData`; empty
  (data-less) variants beyond `NONE` are skipped by the derive.
- **Nested vectors are unsupported** (matches a FlatBuffers format restriction —
  wrap the inner vector in a table).

## Running the tests

```sh
cargo test --features alloc      # primitives, nesting, unions, vectors
cargo test                       # subset that needs no allocator
```

`tests/test_generated.rs` is `flatc`-generated code from `tests/test.fbs`, used to
verify wire compatibility in both directions.

## License

Unspecified.
