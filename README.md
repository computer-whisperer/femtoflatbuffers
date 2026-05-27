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

The crate is `#![no_std]` unconditionally; `alloc` only pulls in `extern crate
alloc`. Dependencies are kept `no_std` too (`thiserror` is used with
`default-features = false`), and CI builds the library for a bare-metal target
(`thumbv7em-none-eabi`) to keep it that way.

## Supported types

- Integers: `u8`/`i8`, `u16`/`i16`, `u32`/`i32`, `u64`/`i64`.
- Floats: `f32`, `f64`.
- `bool`.
- `Option<T>` — maps to FlatBuffers' "field absent" (vtable entry `0`).
- Nested tables via `#[derive(Table)]`.
- Unions via `#[derive(Union)]` on an enum whose first variant is the `NONE`
  marker and whose remaining variants each wrap a single table type.
- Vectors: `Vec<T>` (`alloc`) or `heapless::Vec<T, N>` (`heapless`).
- Strings: `String` (`alloc`) or `heapless::String<N>` (`heapless`).

## Known issues & limitations

This crate was written pre-documentation and has rough edges. As of this review:

- **`&str` is not supported**, only owned strings (`String` / `heapless::String`).
- **`heapless::String` decode is not UTF-8 aware.** It copies bytes into chars
  one-for-one (Latin-1), so non-ASCII text is mangled; the `alloc` `String` impl
  decodes proper UTF-8. Encoding is correct for both.
- **Vtables are not deduplicated.** The canonical format reuses one vtable across
  identical tables; here every table emits its own. Output is still valid, just
  larger.
- **Defaults are not omitted on encode.** femto always writes every non-`Option`
  field, even at the schema default, where the canonical writer omits it. Output
  is valid (and decodes fine), just larger. *Decoding* an omitted field is
  handled correctly: a zero vtable entry or a truncated vtable yields the
  type's default (zero/false) for scalars, `None` for `Option<T>`, and an empty
  collection for vectors/strings.
- **Only the implicit zero/false default is supported.** Schemas that declare a
  non-zero scalar default cannot be expressed (there is no schema annotation in
  the Rust-as-schema model), so an omitted field always decodes to zero/false.
- **Decoded values from untrusted input may be attacker-chosen, but decoding is
  memory-safe.** The decoder is hardened: every read is bounds-checked, all offset
  arithmetic is overflow-checked, allocation and total work are bounded by the
  buffer length, and recursion is capped (`MAX_DEPTH = 64`). So *no* input can
  panic, over-allocate, hang, or overflow the stack — malformed buffers yield
  `Err`. What it does *not* do is fully validate structure, so a crafted buffer
  whose offsets all happen to be in range can still decode to attacker-controlled
  (but bounded, memory-safe) values. Treat decoded data as untrusted, as with any
  parser that lacks a full schema verifier. See `tests/hardening.rs`.
- **Unions:** the `NONE` variant (variant 0) round-trips — it encodes as an absent
  field and decodes from an absent/zero type, including when the whole union field
  is omitted. Data-less variants *beyond* `NONE` are still skipped by the derive
  (encoding them errors); FlatBuffers union members are always tables, so this
  only affects malformed enums.
- **Nested vectors are unsupported** (matches a FlatBuffers format restriction —
  wrap the inner vector in a table).

## Development

```sh
cargo test                       # primitives, scalars, nesting, unions
cargo test --features alloc      # + alloc Vec<T> and String
cargo test --features heapless   # + heapless Vec<T, N> and String<N>

cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

CI (`.github/workflows/ci.yml`) runs `fmt`, `clippy`, and `build` + `test`
across the feature matrix (`no-features` / `alloc` / `heapless` / `all-features`),
plus a bare-metal `no_std` build.

The fixtures under `tests/generated/` are `flatc`-generated from the matching
`*.fbs` schemas (`test`, `string_test`, `scalars_test`) and cross-check wire
compatibility against the official `flatbuffers` crate in both directions. They
live in a subdirectory so cargo does not treat them as standalone test binaries,
and the `#[path]` includes silence lints on them. Note that `cargo fmt` reformats
them, so after regenerating with `flatc`, run `cargo fmt` again.

## License

Unspecified.
