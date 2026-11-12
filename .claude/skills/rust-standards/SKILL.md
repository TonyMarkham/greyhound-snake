---
name: rust-standards
description: Comprehensive Rust coding guidelines. Use when writing, reviewing, or refactoring Rust code. Covers ownership, error handling, async patterns, concurrency, unsafe code, API design, memory optimization, performance, numeric safety, conversions, serde, pattern matching, macros, closures, observability, testing, and common anti-patterns.
---

# Rust coding style

## Files and modules

- One type per file. The file is named after the type, and holds the
  free functions and constants that exist only for that type.
- Related types go in a module directory. Its `mod.rs` holds only the
  module's docs, its `mod` declarations and its re-exports.
- Modules are split by concern, and files stay small. `main.rs` stays
  thin: module declarations, re-exports, argument parsing, config
  loading and dispatch. When a module does more than its name promises,
  the extra logic moves into a module of its own.
- Several binaries in one crate go in `src/bin/`.

## Imports

- No glob imports (`use foo::*`), anywhere, tests and firmware included.
- No `super::` or `self::` paths. Internal imports start at `crate::`.
- One `use` statement per source crate, with every item from that crate
  in nested braces:
  `use crate::{clock::Clocks, gpio::{Input, Pin}};`
- Order: `crate::`, a blank line, workspace crates, a blank line, then
  `std` and the other external crates.

## Visibility

- Default to `pub(crate)`. Use `pub` only for a crate's real API.
- Nothing is made public just so tests, validation or debugging can
  reach it.
- Shape the public API with `pub use` re-exports.

## Ownership and borrowing

- Borrow (`&T`) rather than `.clone()`.
- Accept `&[T]`, not `&Vec<T>`, and `&str`, not `&String`.
- Use `Cow<'a, T>` when ownership is only sometimes needed.
- Shared ownership: `Arc<T>` across threads, `Rc<T>` on one thread.
- Interior mutability: `RefCell<T>` on one thread, `Mutex<T>` across
  threads, `RwLock<T>` when reads far outnumber writes.
- Implement `Copy` for small, simple types. Keep `Clone` explicit where
  copying costs something.
- Move large values instead of copying them; `Box` them if moves are
  expensive.
- Rely on lifetime elision. Write lifetimes only where the compiler
  needs them.

## Errors

- No `unwrap()`, `expect()` or `panic!` outside tests. The workspace
  lints deny `unwrap_used`, `expect_used`, `panic` and `unused_must_use`,
  and `clippy.toml` sets `allow-unwrap-in-tests = true`.
- Each crate has a `thiserror` error enum and a crate-local `Result<T>`
  alias. Each variant carries a message and an `ErrorLocation`, set by
  one `#[track_caller]` constructor per variant, so every error reports
  where it was raised.
- External errors are converted where they occur, with `map_err`. The
  message names the operation and the failing path, offset or input.
  Helpers never take a hand-written "context" string: the location is
  recorded automatically.
- `anyhow` is used only at a binary's top level, never in library code.
- `main` calls `fn run() -> Result<()>`. On an error it prints it with
  `eprintln!("{e}")` (Display, not Debug) and exits with code 1.
- Propagate with `?` once the error is converted.
- Error messages start in lowercase and have no trailing punctuation.
- Never ignore an error silently.

## Values

- No magic values. A number or string that carries meaning is a named
  `SCREAMING_SNAKE` constant in the file of the type that owns it, a
  config value, or a type. A bare literal is fine only when it carries
  no hidden meaning (`0` as a start index).
- A value someone might change between runs (a rate, a period, a count,
  a port, a seed, a path) goes in a toml file the code reads, never in
  code. A fixed fact stays in code as a named constant, with a comment
  saying where it comes from.
- No `as` casts to paper over a conversion. `From` is only for
  widenings that lose nothing on any target. Convert `usize` to `u32`
  with `u32::try_from` and a real error, and do arithmetic that mixes
  types in `u64` via `u64::from`.
- Handle integer overflow explicitly: `checked_`, `saturating_`,
  `wrapping_` or `overflowing_`.
- Never compare floats with `==`. Compare within a tolerance, and order
  them with `total_cmp`.
- Bound values with `clamp` and saturating arithmetic.
- Use the `NonZero*` types where zero is not allowed.
- Use `const` for an inlined value and `static` for one instance with
  an address. Make functions `const fn` where they can run at compile
  time, use inline `const { }` blocks for compile-time checks, and
  const generics (`<const N: usize>`) to parameterise over values.

## Types and API design

- Wrap IDs and other look-alike values in newtypes (`UserId(u64)`), and
  validate in the newtype's constructor.
- Parse into validated types at the boundary, then trust them.
- Use enums for mutually exclusive states, and the typestate pattern to
  enforce a state machine at compile time.
- `Option<T>` for a value that may be absent, `Result<T, E>` for an
  operation that can fail, `!` for a function that never returns.
- No stringly typed APIs: use enums, newtypes or validated types.
- Use `PhantomData` to express a type relationship at no runtime cost,
  and `#[repr(transparent)]` for newtypes that cross an FFI boundary.
- Implement `Deref`/`DerefMut` only for smart pointers and transparent
  wrappers.
- `Display` is for users, `Debug` for diagnostics; never swap them.
  Numeric newtypes implement `LowerHex`, `UpperHex`, `Octal` and
  `Binary`.
- Use a builder for complex construction, with `#[must_use]` on its
  methods.
- Mark types and functions `#[must_use]` when ignoring the result is
  likely a bug.
- Use `#[non_exhaustive]` on public enums and structs that may grow.
- Implement `From`, never `Into` (error conversions excepted: those use
  `map_err`, see *Errors*). Accept `impl Into<T>` for flexible inputs
  and `impl AsRef<T>` / `impl AsMut<T>` when you only borrow.
- Implement `TryFrom` for fallible conversions and `FromStr` for
  parsing from a string.
- Implement `Default` where a sensible default exists, and the standard
  traits (`Debug`, `Clone`, `PartialEq`, …) on public types.
- Collection types implement `FromIterator`, `Extend`, and
  `IntoIterator` for all three reference forms.
- Overload operators only where the meaning is natural.
- Sealed traits stop outside implementations; extension traits add
  methods to outside types.
- Library crates make serde an optional feature.
- Don't over-abstract with generics, and don't use `Box<dyn Trait>`
  where `impl Trait` works.

## Traits and generics

- An associated type when each impl has exactly one output; a generic
  parameter when a type can implement the trait for many inputs.
- Blanket impls (`impl<T: Bound> Trait for T`) give behaviour to every
  type with the bound.
- Respect the orphan rule: wrap a foreign type in a newtype to give it
  a foreign trait.
- Define a trait by a few required methods plus defaulted ones built on
  them.
- Choose static (`impl Trait`, generics) or dynamic (`dyn Trait`)
  dispatch on purpose, and keep a trait object-safe when it is used as
  `dyn`.
- Put trait bounds only where needed, in `where` clauses.

## Pattern matching and closures

- `let … else` for early-return extraction, `matches!()` for boolean
  pattern tests, `if let` chains to combine bindings and conditions,
  `@` bindings to capture a value while matching it.
- Match your own enums exhaustively; no catch-all `_` that hides a new
  variant.
- A callback asks for the least restrictive `Fn` trait it needs.
- Return closures as `impl Fn…`, not `Box<dyn Fn>`. Accept `impl Fn`
  for hot callbacks, `&dyn Fn` / `Box<dyn Fn>` to cut code size or to
  store them.
- Use `move` for closures that outlive their scope; clone first to keep
  the original. Capture only what the closure uses.

## Collections

- Default to `Vec`. `VecDeque` for a queue, never `LinkedList`.
- `HashMap` for speed, `BTreeMap` for order or ranges, `IndexMap` for
  insertion order.
- `HashSet`/`BTreeSet` for membership and de-duplication, not
  `Vec::contains`.
- `BinaryHeap` for a priority queue.

## Macros

- Use a macro only when a function or generic can't do the job.
- In `macro_rules!`, rely on hygiene, use `$crate` for paths, and
  capture with precise fragment specifiers rather than `:tt`.
- Export declarative macros with `#[macro_export]`. Hide their helper
  items in a `#[doc(hidden)] pub mod __private`.
- Procedural macros live in their own `proc-macro = true` crate, are
  built with `syn`, `quote` and `proc-macro2`, and report errors as
  spanned compile errors, never by panicking.

## Serde

- Match the external names with `#[serde(rename_all = …)]`.
- `#[serde(default)]` for optional and backward-compatible fields;
  `skip_serializing_if` to omit empty ones.
- `#[serde(flatten)]` to inline nested structs or collect extra keys;
  `#[serde(deny_unknown_fields)]` to reject unexpected keys.
- Choose enum tagging on purpose (external, internal, adjacent,
  untagged).
- Customise one field with `with` / `serialize_with` /
  `deserialize_with`; validate while deserialising with
  `#[serde(try_from = "Raw")]`.

## Naming

- `UpperCamelCase` for types, traits, enums and variants; `snake_case`
  for functions, methods, variables and modules; `SCREAMING_SNAKE_CASE`
  for constants and statics.
- Short lifetimes (`'a`, `'de`, `'src`) and single-letter type
  parameters (`T`, `E`, `K`, `V`).
- Conversion prefixes: `as_` is free, `to_` is expensive, `into_`
  consumes.
- No `get_` prefix on simple getters. `is_`, `has_`, `can_`, `should_`
  for booleans.
- Iterator methods are `iter()`, `iter_mut()` and `into_iter()`, and
  iterator types are named after the method that makes them.
- Acronyms are words: `HttpServer`, not `HTTPServer`.
- No `-rs` or `-rust` suffix on crate names.

## Memory and performance

- Profile before optimising.
- `with_capacity()` when the size is known. Reuse collections in loops
  (`clear`, `drain`, `clone_from`, `collect_into`); batch insert with
  `extend`.
- `SmallVec` for usually-small collections, `ArrayVec<T, N>` for fixed
  capacity with no heap, `Box<[T]>` for fixed-size heap data,
  `ThinVec<T>` for often-empty collections, compact string types where
  memory is tight.
- Box large enum variants. Use the smallest integer type that fits.
  Guard type sizes with static assertions.
- `mem::take` / `mem::replace` to move a value out of a `&mut` without
  cloning. Know the drop order: fields top to bottom, locals in reverse.
- Avoid `format!()` where a literal works, and never in hot paths;
  `write!()` into an existing buffer instead.
- Zero-copy with slices and `Bytes`; arena allocators for batches.
- Prefer iterators to indexing. Keep them lazy, collect once, and
  avoid `chain` in hot loops.
- Use the entry API for insert-or-update.
- `ahash` / `FxHashMap` when DoS resistance isn't needed.
- Wrap `Read`/`Write` in `BufReader`/`BufWriter` for many small
  operations.
- `#[inline]` for small hot functions; `#[inline(always)]` only where
  profiling proves it. `#[cold]` and `#[inline(never)]` for error paths
  and rarely run code.
- Write hot loops so bounds checks can be eliminated, lay data out for
  the cache, and use portable SIMD for vector work.
- Release builds: LTO, `codegen-units = 1`; PGO and
  `target-cpu=native` where the deployment target is known.

## Unsafe

- A `// SAFETY:` comment above every `unsafe` block and a `# Safety`
  section on every `unsafe fn`.
- Keep `unsafe` blocks as small as the operation that needs them.
- Run `cargo miri test` for every crate with `unsafe` code.
- `MaybeUninit<T>` for uninitialised memory; never
  `mem::uninitialized()` or `mem::zeroed()` for types with validity
  invariants.
- Edition 2024: `unsafe extern { }` with each item marked `safe` or
  `unsafe`, and `#[unsafe(no_mangle)]`, `#[unsafe(export_name = …)]`,
  `#[unsafe(link_section = …)]`.
- Let the compiler derive `Send`/`Sync`; document the invariants when
  implementing them by hand.

## Async and concurrency

- Blocking work runs on its own thread (or `spawn_blocking`). Async work
  runs on tokio. No blocking I/O inside an async task: `tokio::fs`, not
  `std::fs`.
- Configure the tokio runtime for the workload.
- Never hold a `Mutex`/`RwLock` across `.await`; clone `Arc`/`Rc` data
  before an await point.
- `join!` / `try_join!` for independent futures, `select!` to race
  them, with only cancellation-safe futures in `select!` branches.
- Channels: bounded for backpressure, `mpsc` for queues, `broadcast`
  for pub/sub, `watch` for the latest value, `oneshot` for one reply.
- `JoinSet` for a changing set of tasks; `CancellationToken` for
  shutdown.
- Native `async fn` in traits, not `async_trait`; `AsyncFn` bounds
  instead of `F: Fn() -> Fut`.
- rayon's `par_iter()` for CPU-bound data parallelism;
  `std::thread::scope` to borrow stack data across threads.
- The weakest correct `Ordering` on every atomic. `thread_local!` with
  `Cell`/`RefCell`, never `static mut`.

## Logging

- Logging goes through `tracing`, never `println!`.
- Libraries emit through the facade and never install a subscriber.
- Record structured fields, not values formatted into the message.
- Attach context with `#[tracing::instrument]` and spans.
- Use levels on purpose, filtered with `EnvFilter` / `RUST_LOG`.
- Log each error once, with its full source chain. Never log secrets or
  personal data.

## Comments and docs

- A comment states the invariant, a rule the reader can check by hand,
  or the reason behind a choice that isn't obvious. It never repeats
  what the code already says.
- Every public item has a doc comment (`#![deny(missing_docs)]` in
  library crates). Module docs use `//!`.
- A fact taken from a datasheet, manual or specification cites the
  section it comes from.
- Doc examples compile. Examples that need hardware are marked
  `no_run`.
- Doc sections: `# Examples` with runnable code, `# Errors` for
  fallible functions, `# Panics` where a function can panic,
  `# Safety` for `unsafe fn`.
- Examples use `?`, not `.unwrap()`, and hide their setup lines with
  `# `.
- Link related items with intra-doc links.
- A published crate fills in its `Cargo.toml` metadata, and can share
  its README with the crate docs through
  `#![doc = include_str!("../README.md")]`.

## Cargo

- Edition 2024, stable toolchain.
- Dependency versions and features are declared once, in the
  workspace's `[workspace.dependencies]`. A member crate uses
  `name = { workspace = true }`, inherits `version` and `edition` from
  `[workspace.package]`, and sets `[lints] workspace = true`. Never put
  a version in a member crate.
- Every new dependency has its licence checked (MIT or Apache-2.0
  unless the owner approves an exception).
- Declare `rust-version` (MSRV) and test it.
- Cargo features are strictly additive.
- `build.rs` stays minimal, deterministic and idempotent.
- Lints, set at the workspace level: deny `clippy::correctness`; warn
  on `clippy::suspicious`, `style`, `complexity` and `perf`; enable
  `clippy::pedantic` and `clippy::nursery` lints selectively;
  `clippy::cargo` for published crates; `unexpected_cfgs` with the
  known cfgs declared.

Release profile settings:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.bench]
inherits = "release"
debug = true
strip = false

[profile.dev]
opt-level = 0
debug = true

[profile.dev.package."*"]
opt-level = 3  # optimise dependencies in dev builds
```

## Tests

- Unit tests never go inside source files. They live in `src/tests/`,
  wired in by `#[cfg(test)] mod tests;` in the crate root and split
  into files by behaviour.
- Integration tests go in the crate's `tests/` folder.
- Tests go through the crate's own (`pub(crate)`) API.
- Test names follow `given_<setup>_when_<action>_then_<expectation>`,
  and the body is marked with `// given`, `// when` and `// then`.
- Put dependencies behind traits so tests can mock them (`mockall`).
- Clean up test fixtures with RAII (`Drop`).
- `#[tokio::test]` for async tests; `#[should_panic]` to test a panic.
- `proptest` for properties, `insta` for snapshots of complex output,
  `loom` for lock-free and concurrent code.
- `criterion` for benchmarks, with `black_box` around inputs.
- Keep doc examples as doctests.

## Before saying Rust work is done

Run these from each workspace root. Firmware crates need their board
target, so run them from the firmware crate's own folder:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets --no-deps -- -D warnings
    cargo test --workspace
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Every command must pass with no warnings. `cargo doc` is there because
broken doc links never show up in `cargo check` or `cargo clippy`.
Firmware targets can't run tests, so their tests run on the host. Say
which commands you actually ran: reading the code is not checking it.
