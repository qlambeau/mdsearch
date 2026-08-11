# Rust Engineering Constitution

**Status:** Normative. **Audience:** human and agentic contributors. **Applies to:** all Rust code in this repository.

This document is the highest-precedence engineering authority in the repository. It governs *how* code is written. Specifications govern *what* is built. When a specification and this constitution conflict, the constitution wins on form and the specification wins on behaviour — and the conflict MUST be raised, not silently resolved.

---

## 0. How to use this document

### 0.1 Rule grammar

| Keyword | Meaning |
| --- | --- |
| **MUST** / **MUST NOT** | Non-negotiable. A violation blocks merge. |
| **SHOULD** / **SHOULD NOT** | Strong default. Deviation requires a code comment naming the rule ID and the reason. |
| **MAY** | Permitted, no justification needed. |

Every rule has a stable ID (`R-<AREA>-<NN>`). Cite rule IDs in commit messages, review comments, and deviation comments. IDs are append-only: a retired rule is marked `RETIRED`, never renumbered or reused.

### 0.2 Recording a deviation

A `SHOULD` may be broken inline. A `MUST` may not — it requires an ADR.

```rust
// DEVIATION(R-ASY-03): this lock is held across .await because the
// upstream C FFI handle is not Send. Contained to this function.
```

### 0.3 Rules for the agent

- **R-AGT-01** — An agent MUST read this document before its first edit in a session, and MUST NOT rely on a summary of it.
- **R-AGT-02** — An agent MUST NOT introduce a new crate-level dependency, a new workspace member, or a new architectural layer without explicit human approval in the current session.
- **R-AGT-03** — An agent MUST NOT weaken a test, delete an assertion, add `#[ignore]`, or loosen a lint to make a build pass. If the code cannot satisfy the test, stop and report.
- **R-AGT-04** — An agent MUST NOT write `unsafe`. `unsafe` is a human-authored, human-reviewed construct (see §8).
- **R-AGT-05** — An agent MUST leave the working tree passing §13 gates at the end of every unit of work. "I'll fix the tests next" is not a valid end state.
- **R-AGT-06** — When a specification is ambiguous, an agent MUST ask rather than choose. Guessing at behaviour is the single most expensive failure mode in spec-driven development.
- **R-AGT-07** — An agent MUST NOT mark work complete based on intent. Completion requires the §14 checklist to have actually been executed, with command output observed.

---

## 1. Spec-driven development loop

Code exists to satisfy a specification. The loop is fixed:

```
SPEC  →  CONTRACT  →  RED TEST  →  IMPLEMENTATION  →  GREEN  →  REFACTOR  →  TRACE
```

- **R-SDD-01** — No production code is written before a specification exists for the behaviour it implements. A specification MAY be a single paragraph, but it MUST be written down and MUST be addressable by an ID.
- **R-SDD-02** — Every specified behaviour MUST be traceable to at least one test. Encode the link in the test name or in a doc comment on the test:
  ```rust
  /// Covers: SPEC-AUTH-014 — expired tokens are rejected before signature checks.
  #[test]
  fn rejects_expired_token_without_verifying_signature() { /* ... */ }
  ```
- **R-SDD-03** — Every public type, trait, and function MUST be attributable to a specification. Speculative abstraction ("we'll need this later") is forbidden; it is the primary source of dead weight in agent-written codebases.
- **R-SDD-04** — Types are the first line of the specification. Before writing a validation function, ask whether the type system can make the invalid state unrepresentable (see §7).
- **R-SDD-05** — A behavioural change MUST update the specification in the same commit as the code. Documentation drift is a defect of equal severity to a logic defect.
- **R-SDD-06** — When implementation reveals the specification is wrong, stop. Fix the specification first, then resume. Never let code silently become the spec of record.

---

## 2. Naming conventions

### 2.1 Casing (mechanical, enforced by `clippy::pedantic`)

| Item | Convention | Example |
| --- | --- | --- |
| Crate, module, file | `snake_case` | `order_pricing`, `token_store.rs` |
| Type, trait, enum variant | `UpperCamelCase` | `OrderId`, `TokenStore`, `Expired` |
| Function, method, local, field | `snake_case` | `settle_invoice`, `retry_budget` |
| Constant, static | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT` |
| Lifetime | short lowercase | `'a`, `'src`, `'tx` |
| Generic type parameter | single capital or `UpperCamelCase` | `T`, `E`, `Store` |
| Feature flag | `kebab-case` | `postgres-store` |

- **R-NAM-01** — Acronyms are treated as words: `HttpClient`, `parse_url`, `OauthToken`. Never `HTTPClient` or `parseURL`.
- **R-NAM-02** — Do not stutter. Inside `token::` the type is `token::Store`, not `token::TokenStore`. The path carries the context.
- **R-NAM-03** — Never abbreviate a domain term. `subscription`, not `sub`. Abbreviate only universally-understood mechanics: `ctx`, `cfg`, `id`, `db`, `tx`, `req`, `res`, `i`/`j` for indices.

### 2.2 Method naming (idiomatic Rust semantics carry meaning)

- **R-NAM-04** — Conversion prefixes are load-bearing and MUST NOT be misused:
  - `as_*` — cheap borrow-to-borrow, no allocation, infallible. `as_str`.
  - `to_*` — expensive and/or allocating, infallible. `to_owned`, `to_string`.
  - `into_*` — consumes `self`, transfers ownership. `into_inner`.
  - `try_into_*` / `try_from` — consumes and may fail. Returns `Result`.
- **R-NAM-05** — Getters take no `get_` prefix: `fn name(&self) -> &str`. Reserve `get_` for genuinely indexed or fallible lookup (`get_mut`, `get(index)`).
- **R-NAM-06** — Predicates read as assertions and return `bool`: `is_expired`, `has_capacity`, `can_settle`. Never `check_*` or `validate_*` for something that returns `bool`.
- **R-NAM-07** — Constructors: `new` for the single obvious construction, `with_*` for a variant (`with_capacity`), `from_*` for conversion, `try_new` when construction can fail. A type with more than four construction parameters SHOULD use a builder.
- **R-NAM-08** — Fallible operations that are the fallible form of an infallible method take the `try_` prefix: `reserve` / `try_reserve`.
- **R-NAM-09** — Iterator triplet is fixed: `iter()` → `&T`, `iter_mut()` → `&mut T`, `into_iter()` → `T`.

### 2.3 Domain naming

- **R-NAM-10** — The domain layer uses the ubiquitous language of the specification verbatim. If the spec says "settlement", the code says `settlement` — not `payment`, not `transaction`. A rename in the spec is a rename in the code, in the same commit.
- **R-NAM-11** — Names MUST NOT encode their technical layer inside the domain. `OrderService`, `OrderManager`, `OrderHelper`, `OrderUtil`, and `OrderData` are all banned in `domain`. Name the behaviour: `OrderPricing`, `SettlementPolicy`, `RefundCalculator`.
- **R-NAM-12** — `util`, `common`, `helpers`, `misc`, `shared`, and `core` are forbidden as module or crate names. They are landfills. Name the capability instead.
- **R-NAM-13** — Traits are named for the capability they confer, not the shape of the implementation. Prefer a noun-of-role (`TokenStore`, `Clock`) or a verb-able (`Serialize`, `Sortable`). Never suffix `Interface`, `Impl`, `Trait`, or prefix `I`.
- **R-NAM-14** — Concrete implementations of a port are named for their technology, not for the port: `PostgresOrderRepository`, `InMemoryOrderRepository`, `SystemClock` — implementing `OrderRepository` and `Clock`. Never `OrderRepositoryImpl`.

---

## 3. Repository and directory organization

### 3.1 Repository and Rust workspace layout

This repository currently contains the SDD Workflow Kit and uses the following
specification layout:

```
.
├── AGENTS.md
├── README.md
├── SDD_WORKFLOW.md
├── .agents/
│   └── skills/
└── specs/
    ├── CONSTITUTION.md
    ├── prds/
    ├── adr/
    ├── templates/
    ├── NNN-feature-slug/
    └── archive/
```

When the Rust implementation workspace is introduced, it follows the layout
below. Layering is enforced by the compiler, not by convention. A layer is a
crate; an illegal dependency is a compile error.

```
.
├── Cargo.toml                  # [workspace] — members, resolver = "3", shared [workspace.dependencies]
├── Cargo.lock                  # committed (R-DEP-02)
├── rust-toolchain.toml         # pinned toolchain (R-TOOL-01)
├── rustfmt.toml
├── clippy.toml
├── deny.toml                   # cargo-deny: licences, advisories, bans
├── specs/
│   ├── CONSTITUTION.md         # this file
│   ├── prds/                   # PRD-NNN.md — product requirements
│   ├── adr/                    # ADR-NNN.md — architecture decisions
│   ├── templates/              # source templates, not active specifications
│   └── NNN-feature-slug/        # feature packet and executable scenarios
├── crates/
│   ├── domain/                 # layer 0 — pure. zero I/O, zero async, zero frameworks.
│   ├── application/            # layer 1 — use cases + PORT TRAITS. depends on domain only.
│   ├── adapters/               # layer 2 — one crate per technology, implements ports.
│   │   ├── store-postgres/
│   │   ├── http-client/
│   │   └── queue-kafka/
│   ├── infrastructure/         # layer 2 — cross-cutting: telemetry, config loading.
│   └── app/                    # layer 3 — the ONLY place composition happens.
│       ├── src/lib.rs          # wiring / composition root
│       └── src/bin/server.rs   # thin binary: parse args, build, run
├── tests/                      # workspace-level end-to-end tests (few, slow, high value)
└── xtask/                      # cargo-xtask: repo automation in Rust, not shell
```

- **R-DIR-01** — The dependency graph is acyclic and points inward only:
  ```
  app  →  adapters  →  application  →  domain
   └──────────────────────┴───────────────┘
  ```
  `domain` depends on nothing in the workspace. `application` depends only on `domain`. Adapters depend on `application` (for the port traits) and `domain` (for the types). **Nothing depends on an adapter except `app`.**
- **R-DIR-02** — `domain` MUST NOT depend on `tokio`, `axum`, `sqlx`, `reqwest`, `serde` derive on domain types, or any other I/O or framework crate. Permitted: `std`, and pure computation crates (`rust_decimal`, `time` without feature flags that pull I/O). Enforce with `cargo-deny` bans scoped to the crate.
- **R-DIR-03** — Composition (constructing concrete adapters and injecting them) happens **only** in `crates/app`. If any other crate names a concrete adapter type, the layering is broken.
- **R-DIR-04** — A binary in `src/bin/` MUST NOT exceed ~50 lines. It parses configuration, calls a `run(...)` in a library crate, and maps the result to an exit code. Binaries are untestable; libraries are testable.
- **R-DIR-05** — Prefer `foo.rs` + `foo/` sibling layout (Rust 2018 style) over `foo/mod.rs`. One declaration site per module tree.
- **R-DIR-06** — One primary concept per file. A file over **400 lines** of non-test code is a review trigger; over **700**, it MUST be split. Test modules do not count toward the limit.
- **R-DIR-07** — `mod.rs` / module-root files contain declarations and re-exports only. No logic in a module root.
- **R-DIR-08** — Module structure follows the domain, never the technical kind. `crates/domain/src/billing/`, not `crates/domain/src/structs/` or `src/traits/`.
- **R-DIR-09** — Shared code between two crates that has no home does not go in a `common` crate (R-NAM-12). Either it belongs to `domain`, or it is a genuine capability and gets a named crate.
- **R-DIR-10** — Dependency versions are declared once in `[workspace.dependencies]`; members use `dep = { workspace = true }`. Version drift between members is a defect.

### 3.2 Inside a crate

```
crates/application/
├── Cargo.toml
├── src/
│   ├── lib.rs               # module decls + the crate's curated public API
│   ├── error.rs             # this crate's error enum
│   ├── ports/               # PORT TRAITS — the seams (§5)
│   │   ├── mod.rs
│   │   ├── order_repository.rs
│   │   └── clock.rs
│   └── settle_order/        # one module per use case
│       ├── mod.rs
│       ├── command.rs       # input DTO
│       └── handler.rs       # orchestration + #[cfg(test)] unit tests
└── tests/
    └── settle_order.rs      # integration tests against the public API
```

- **R-DIR-11** — `lib.rs` contains no logic. It declares modules and curates the public surface with explicit `pub use`.
- **R-DIR-12** — Re-export the public API deliberately. Callers write `application::SettleOrder`, not `application::settle_order::handler::SettleOrder`. Internal module paths are not part of the contract.
- **R-DIR-13** — Default visibility is private. Escalate one level at a time: private → `pub(super)` → `pub(crate)` → `pub`. Every `pub` on a non-API item is a bug.

---

## 4. Separation of concerns

### 4.1 The four responsibilities

Each layer has exactly one job. Mixing them is the defect this section exists to prevent.

| Layer | Owns | MUST NOT contain |
| --- | --- | --- |
| **domain** | Entities, value objects, invariants, pure business rules and calculations | I/O, `async`, SQL, HTTP, clocks, RNG, env vars, logging, serde wire formats |
| **application** | Use-case orchestration, transaction boundaries, port trait definitions, authorization decisions | Business rules, SQL, HTTP types, concrete adapter types |
| **adapters** | Translation between the outside world and port traits; wire formats; retries | Business rules, decisions the domain should make |
| **app** | Configuration, wiring, process lifecycle, shutdown | Anything else |

- **R-SEP-01** — Business rules live in `domain`. If a rule can be stated without mentioning a database, a queue, or a request, it is a domain rule and it MUST NOT live in an adapter or handler.
- **R-SEP-02** — The domain is deterministic and pure. **All** nondeterminism — time, randomness, UUID generation, environment, filesystem, network — enters through a port (§5). A domain function given the same inputs MUST return the same output, forever.
  ```rust
  // WRONG — domain reaches for ambient state
  impl Subscription {
      pub fn is_expired(&self) -> bool { Utc::now() > self.expires_at }
  }

  // RIGHT — time is an input
  impl Subscription {
      pub fn is_expired(&self, now: Timestamp) -> bool { now > self.expires_at }
  }
  ```
- **R-SEP-03** — A use-case handler orchestrates; it does not compute. Its body reads as: load via port → call domain → persist via port → return. Non-trivial branching logic inside a handler is misplaced domain logic.
- **R-SEP-04** — Adapters are thin and dumb. An adapter maps types and performs I/O. An `if` in an adapter that expresses a business condition is a layering violation.
- **R-SEP-05** — Wire types and domain types are distinct. HTTP DTOs and database row structs live in the adapter that owns them and are converted at the boundary via `TryFrom`. A domain type MUST NOT derive `Serialize`/`Deserialize` for an external format — that couples your business model to a wire contract you don't control.
- **R-SEP-06** — Transaction boundaries are declared by the application layer and executed by the adapter. The domain never knows a transaction exists.
- **R-SEP-07** — A single function does one of: **decide**, **compute**, or **perform I/O**. Never two. This is what makes the decision logic unit-testable without a runtime.
- **R-SEP-08** — Push I/O to the edges; keep the middle pure ("functional core, imperative shell"). Read everything you need, compute the decision purely, then write. Do not interleave.
- **R-SEP-09** — No global mutable state. No `static mut`, no `lazy_static` mutable singleton, no ambient service locator. Dependencies are passed explicitly. This is not stylistic — hidden state makes parallel tests flaky.
- **R-SEP-10** — Configuration is read once, at startup, in `app`, and passed down as typed values. No `std::env::var` outside `app`.

### 4.2 Function and type discipline

- **R-SEP-11** — A function SHOULD fit on one screen (~40 lines) and MUST NOT exceed 80 lines of body. Length is a proxy for the real problem: it is doing more than one thing.
- **R-SEP-12** — Cyclomatic complexity per function SHOULD stay ≤ 10. Enforce with `clippy::cognitive_complexity`.
- **R-SEP-13** — More than 5 parameters means a missing parameter struct. More than 3 with the same type means a missing newtype (R-TYP-01).
- **R-SEP-14** — Accept the most general type you can use: `&str` over `&String`, `&[T]` over `&Vec<T>`, `impl IntoIterator<Item = T>` over `Vec<T>` when you only iterate.
- **R-SEP-15** — Return concrete types from public APIs. Prefer a named type or `impl Trait` over `Box<dyn Trait>` on the return path unless heterogeneity is genuinely required.

---

## 5. Ports and traits — abstraction at the seams

**The stance:** traits are for *seams*, not for *decoration*. Rust's compiler already gives you encapsulation and change-detection; a trait's job here is to invert a dependency so the pure core does not know about the impure world. A trait with exactly one implementation that will never grow one is negative value — it adds indirection, defeats inlining, and obscures the call graph. Trait-per-struct is a Java reflex, not a Rust one.

### 5.1 When a trait is mandatory

- **R-TRT-01** — Every crossing of the process boundary MUST be behind a port trait defined in `application::ports`: databases, HTTP calls, message queues, filesystem, clock, randomness, ID generation, secrets, email/SMS, feature flags, payment providers.
- **R-TRT-02** — Every source of nondeterminism MUST be behind a port, however trivial. `Clock` and `IdGenerator` are the two most valuable traits in most codebases; without them, half your tests need `sleep` or accept unpredictable output.
- **R-TRT-03** — Any point where the specification anticipates more than one strategy MUST be a trait (e.g. `PricingStrategy`, `NotificationChannel`).
- **R-TRT-04** — Ports are **defined by the consumer, in the inner layer** (dependency inversion). `application` owns `OrderRepository`; the Postgres adapter implements it. Never the reverse — that would invert the dependency arrow and break R-DIR-01.

### 5.2 When a trait is forbidden

- **R-TRT-05** — MUST NOT define a trait for a pure, deterministic, in-process type. Value objects, entities, and calculators are concrete. There is nothing to fake — call them directly.
- **R-TRT-06** — MUST NOT define a trait with one implementation and no second implementation in prospect, *unless* it exists to satisfy R-TRT-01/02 (in which case the test double is the second implementation, and that counts).
- **R-TRT-07** — MUST NOT create a trait purely to enable mocking of code you own and that is already pure. If a unit is hard to test without a mock, the fault is usually R-SEP-07, not a missing trait. Fix the seam, don't mock the mess.

### 5.3 Designing a port

- **R-TRT-08** — Ports are narrow and role-based. Prefer three focused traits over one twelve-method `Repository`. A test double for a fat trait is mostly `unimplemented!()`, which is a design smell made visible.
- **R-TRT-09** — Port signatures speak the domain's language: domain types in, domain types out, domain errors on failure. No `sqlx::Error`, no `reqwest::Response`, no `serde_json::Value` in a port signature — that leaks the technology the port exists to hide.
- **R-TRT-10** — Port methods return `Result<_, E>` with an error type owned by the port's layer. Adapters map their technology errors into it (§6).
- **R-TRT-11** — Ports MUST be object-safe unless there is a measured reason not to. Keep them free of generic methods, `Self`-by-value returns, and associated constants so `dyn Port` stays available.
- **R-TRT-12** — Async ports use `async fn` in traits (Rust ≥ 1.75) for statically-dispatched use, or `#[trait_variant::make(Send)]` / `Pin<Box<dyn Future>>` when `dyn` dispatch is required. Choose one convention per workspace and record it in an ADR.
- **R-TRT-13** — Async ports MUST declare `Send + Sync + 'static` bounds where the runtime requires them. Do not discover this at the call site.
- **R-TRT-14** — Traits carry documented contracts, not just signatures. Document invariants, error semantics, idempotency, and ordering guarantees on the **trait**; implementations inherit the obligation. An undocumented trait contract will be implemented inconsistently.
- **R-TRT-15** — Default methods only for genuine convenience derived from required methods. Never put behaviour an implementor is expected to override in a default — make it required.

### 5.4 Injection

- **R-TRT-16** — Dependencies are injected via constructor. No service locator, no global registry, no `OnceCell` singleton.
- **R-TRT-17** — Default to **generics** (`struct SettleOrder<R: OrderRepository, C: Clock>`) for static dispatch and inlining. Use `Arc<dyn Port>` when the type must be erased (heterogeneous collections, plugin registries, runtime selection, or when monomorphization bloat is measured).
- **R-TRT-18** — Shared ports across async tasks are `Arc<dyn Port + Send + Sync>`. Never `Arc<Mutex<dyn Port>>` — a port implementation is responsible for its own interior synchronization.
- **R-TRT-19** — Generic parameter count per type SHOULD stay ≤ 3. Beyond that, bundle collaborators into a named context struct.
- **R-TRT-20** — Every port trait MUST ship with at least one in-memory test double in the same crate, gated so tests can use it:
  ```rust
  // crates/application/src/ports/order_repository.rs
  #[cfg(any(test, feature = "test-doubles"))]
  pub mod fake {
      use super::*;
      /// Deterministic in-memory OrderRepository. Honours the same contract as production impls.
      #[derive(Default, Clone)]
      pub struct InMemoryOrderRepository { /* ... */ }
  }
  ```
- **R-TRT-21** — Prefer hand-written fakes over generated mocks. A fake that actually behaves (an in-memory map) tests behaviour; a mock that asserts call order tests your implementation's internals and calcifies refactoring. Use `mockall` only for verifying that an interaction *happened* when that interaction is the specified behaviour.

---

## 6. Error handling

- **R-ERR-01** — `Result` for expected failure, `panic!` for programmer error only. A malformed user input is a `Result`. A violated internal invariant MAY panic.
- **R-ERR-02** — `unwrap()` and `expect()` are forbidden in production code paths. Permitted in tests, in `const` contexts, and where a comment proves infallibility:
  ```rust
  // SAFETY-OF-UNWRAP: regex is a compile-time literal, validated by the `valid_regex` test.
  ```
  Enforce with `#![deny(clippy::unwrap_used, clippy::expect_used)]` in library crates.
- **R-ERR-03** — Libraries define typed errors with `thiserror`. Applications (`app`, binaries) MAY use `anyhow`/`eyre` at the top level only. `anyhow::Error` MUST NOT appear in a library's public signature or in a port trait — it destroys the caller's ability to match on failure.
- **R-ERR-04** — Each crate owns its error enum in `error.rs`. Variants are named for the *situation*, not the source: `OrderNotFound`, `InsufficientFunds` — not `SqlxError`.
- **R-ERR-05** — Error enums are `#[non_exhaustive]` if the crate is consumed outside the workspace.
- **R-ERR-06** — Preserve the cause chain with `#[from]` / `#[source]`. Never stringify an error to pass it along.
- **R-ERR-07** — Error messages are lowercase, no trailing period, and describe the failure with context: `"order {id} not found"`. Not `"Error!"`.
- **R-ERR-08** — Adapters translate technology errors at the boundary. A `sqlx::Error` MUST NOT escape the adapter crate.
- **R-ERR-09** — `?` for propagation. Never `match` a `Result` only to re-return it.
- **R-ERR-10** — Never silently discard an error. `let _ = fallible();` requires a comment explaining why the failure is genuinely irrelevant.
- **R-ERR-11** — Distinguish retryable from terminal failures in the error type when the specification cares. Retry policy lives in the adapter; the decision about whether retrying is meaningful lives in the type.
- **R-ERR-12** — Errors are logged **once**, at the boundary that handles them. Log-and-rethrow produces duplicate noise and destroys signal.

---

## 7. Type design — make invalid states unrepresentable

- **R-TYP-01** — Newtype every domain identifier and constrained primitive. `struct OrderId(Uuid)`, `struct Email(String)`, `struct Quantity(u32)`. Bare `String` and `u64` as domain concepts are forbidden — they permit `charge(customer_id, order_id)` to compile with the arguments swapped.
- **R-TYP-02** — Validate once, at construction, in a `TryFrom`/`try_new`. Downstream code receives the type and MUST NOT re-validate. *Parse, don't validate.*
- **R-TYP-03** — A type with an invariant keeps its fields private and exposes accessors. A `pub` field is a public promise that any value of that type is valid.
- **R-TYP-04** — Model alternatives with enums, not with `Option` fields or boolean flags. Three `Option` fields where exactly one is `Some` is an enum wearing a disguise.
- **R-TYP-05** — Booleans in signatures are forbidden where they select behaviour. `send(email, true)` is unreadable; use a two-variant enum.
- **R-TYP-06** — Prefer `Option<T>` over sentinels (`-1`, `""`, `0`) and `NonZeroU32`/`NonEmptyVec` where the invariant is real.
- **R-TYP-07** — Use typestate for illegal-transition prevention when a lifecycle is specified: `Order<Draft>` → `Order<Submitted>`. Do not apply speculatively (R-SDD-03).
- **R-TYP-08** — Derive the standard set deliberately: `Debug` on essentially everything, `Clone` only when cheap or needed, `Copy` only for small plain data, `PartialEq`/`Eq`/`Hash` when the type is compared or keyed. Never `#[derive(Default)]` on a type with an invariant — it manufactures an invalid value.
- **R-TYP-09** — Prefer borrowed parameters; clone at the boundary where ownership genuinely transfers. `.clone()` sprinkled to silence the borrow checker is a design defect, not a fix.
- **R-TYP-10** — `impl Trait` in argument position for simple flexibility; named generics when the type is referenced more than once or in a `where` clause.

---

## 8. Unsafe, panics, and correctness

- **R-UNS-01** — `#![forbid(unsafe_code)]` at the top of every crate. Remove it only with an ADR.
- **R-UNS-02** — Where `unsafe` is unavoidable (FFI), it is confined to a single dedicated adapter crate that exposes a fully safe API.
- **R-UNS-03** — Every `unsafe` block carries a `// SAFETY:` comment stating the invariants relied upon and why they hold. Enforce with `clippy::undocumented_unsafe_blocks`.
- **R-UNS-04** — Unsafe code MUST be exercised under Miri in CI.
- **R-UNS-05** — Integer arithmetic on domain quantities uses `checked_*` / `saturating_*` explicitly. Silent release-mode wrapping on a monetary value is a defect waiting for production traffic.
- **R-UNS-06** — Money is never `f32`/`f64`. Use integer minor units or `rust_decimal`.
- **R-UNS-07** — Indexing (`v[i]`) is forbidden outside tests where the index is not provably in range; use `.get()`.
- **R-UNS-08** — Panics MUST NOT cross an FFI boundary or escape a request handler. The top-level adapter catches and maps to an error response.

---

## 9. Async and concurrency

- **R-ASY-01** — `async` only where there is real I/O concurrency. The `domain` crate is synchronous (R-DIR-02). Do not make a function async because its caller is.
- **R-ASY-02** — One runtime, created once, in `app`. Library crates MUST NOT create a runtime or call `block_on`.
- **R-ASY-03** — Never hold a `std::sync::Mutex` guard across `.await`. Use `tokio::sync::Mutex` when the critical section must span an await, and prefer message passing over shared locks.
- **R-ASY-04** — Never block in an async context. CPU-bound or blocking work goes to `spawn_blocking`.
- **R-ASY-05** — Every spawned task's `JoinHandle` is owned and awaited, or the task is explicitly documented as fire-and-forget with its failure mode described. Silently dropped handles are lost errors.
- **R-ASY-06** — Every external call has a timeout. Every retry has a bounded budget and jittered backoff. Both are configuration, not literals.
- **R-ASY-07** — Cancellation safety is documented on any `async fn` that is not cancel-safe. A `select!` over a non-cancel-safe future is a data-loss bug.
- **R-ASY-08** — Shutdown is graceful and coordinated by `app` via a cancellation token.

---

## 10. Testing

Testing is not a phase; it is the specification made executable. In an agentic workflow it is the *only* reliable signal that generated code does what was asked.

### 10.1 Non-negotiables

- **R-TST-01** — Tests are written **before** the implementation. Red first — a test that has never failed has never been shown to test anything.
- **R-TST-02** — Every public function, every enum variant with behaviour, and every error path MUST have at least one test. Happy path alone is not coverage.
- **R-TST-03** — Every specified behaviour has a test citing its spec ID (R-SDD-02). Every fixed bug gains a regression test that fails before the fix.
- **R-TST-04** — A test MUST be able to fail for exactly one reason. Multiple unrelated assertion groups in one test means multiple tests.
- **R-TST-05** — Tests MUST be deterministic. No wall-clock dependence, no network, no random seeds without pinning, no reliance on execution order, no `sleep` for synchronization. A flaky test is deleted or fixed within one working day — never `#[ignore]`d and forgotten.
- **R-TST-06** — Tests run in parallel by default. `--test-threads=1` anywhere is a defect in the tests, not a configuration need.
- **R-TST-07** — Assert on values and behaviour, never on log output or on private internals.
- **R-TST-08** — No conditional logic in tests. An `if` in a test means it is two tests, and the branch that never runs is a lie.
- **R-TST-09** — `unwrap`/`expect` are fine in tests (R-ERR-02). Prefer `expect("descriptive")` so a failure names itself.

### 10.2 The pyramid

| Tier | Location | Scope | Speed | Share |
| --- | --- | --- | --- | --- |
| Unit | `#[cfg(test)] mod tests` in-file | one function/type, no I/O | µs | ~70% |
| Contract | `tests/` in port-owning crate | every impl of a port, same suite | ms | as needed |
| Integration | `crates/*/tests/*.rs` | crate's public API, fakes at ports | ms | ~25% |
| End-to-end | `/tests/` at workspace root | real adapters via testcontainers | s | ~5% |

- **R-TST-10** — Unit tests live beside the code in `#[cfg(test)] mod tests` and MAY test private functions. Integration tests live in `tests/`, MUST use only the public API, and thereby prove the API is usable.
- **R-TST-11** — Unit tests MUST NOT touch the network, filesystem, database, clock, or RNG. If a unit test needs any of those, either the code violates R-SEP-02 or the test is misclassified.
- **R-TST-12** — The domain crate is tested with **zero** doubles. It is pure; there is nothing to fake. If a domain test needs a mock, the domain is impure — fix the domain.
- **R-TST-13** — Application handlers are tested with in-memory fakes (R-TRT-20), not with a database.
- **R-TST-14** — **Contract tests are mandatory for every port with more than one implementation.** Write the suite once, generically over the trait, and run it against every implementation including the fake. Without this, your fake and your production adapter drift and your fast tests become fiction.
  ```rust
  // crates/application/tests/order_repository_contract.rs
  pub async fn verify_contract<R: OrderRepository>(repo: R) {
      // insert → find → returns the same order
      // find on absent id → Err(OrderNotFound), not Ok(None)
      // insert duplicate id → Err(Conflict)
  }
  ```
  Then in the Postgres adapter's `tests/`: `verify_contract(PostgresOrderRepository::new(pool)).await;`
- **R-TST-15** — End-to-end tests use real dependencies via `testcontainers`, never a shared or developer-local database. They are few and cover only critical paths.

### 10.3 Coverage and rigour

- **R-TST-16** — Line coverage ≥ **85%** workspace-wide and ≥ **95%** in `crates/domain`, measured by `cargo-llvm-cov`. CI fails below threshold. Coverage is a floor for spotting untested regions, not a goal — 100% coverage with weak assertions is worthless.
- **R-TST-17** — Branch/error-path coverage is explicit: every `Err` variant a function can return has a test that produces it.
- **R-TST-18** — Pure functions with algebraic properties (round-trips, invariants, idempotency, ordering) MUST have property tests via `proptest`. Serialization round-trips MUST be property-tested. Failing seeds are committed as regression cases.
- **R-TST-19** — Every parser or any code handling untrusted bytes MUST have a fuzz target (`cargo-fuzz`). The corpus is committed.
- **R-TST-20** — Mutation testing (`cargo-mutants`) SHOULD run on `crates/domain` in CI on a schedule. Surviving mutants indicate assertions that assert nothing — this is the check that coverage cannot give you.
- **R-TST-21** — Doc examples on public API items are compiled and run by `cargo test`. A doc example that cannot compile is documentation that is already wrong. Use `no_run` for I/O examples; never `ignore`.

### 10.4 Structure and readability

- **R-TST-22** — Test names describe behaviour and outcome, not the method under test: `returns_insufficient_funds_when_balance_below_amount`. Never `test_settle_1`.
- **R-TST-23** — Every test has visible Arrange / Act / Assert structure, separated by blank lines. Three sections, in that order.
- **R-TST-24** — Build test data with builders or fixture helpers that default every irrelevant field, so each test names only the values it depends on. Twenty-line struct literals in tests hide the one field that matters.
- **R-TST-25** — Shared test helpers live in `tests/common/mod.rs` or a `test-support` crate gated behind a feature. Never `pub` in production modules.
- **R-TST-26** — Test the observable contract, not the implementation. A refactor that changes no behaviour MUST NOT change a single test. If it does, the tests were over-specified (usually via mocks — see R-TRT-21).
- **R-TST-27** — Table-driven tests via `rstest` for input/expectation matrices. Each case is independently named and independently reported.
- **R-TST-28** — Assertion messages carry context: `assert_eq!(got, want, "settling order {id}")`. Use `pretty_assertions` for diffable failures.
- **R-TST-29** — Snapshot tests (`insta`) for large structured output only. Reviewing a snapshot diff is mandatory — accepting snapshots blindly converts tests into a changelog.
- **R-TST-30** — Test code is production code. It is reviewed, formatted, linted, refactored, and held to the naming rules of §2. Duplication in tests is tolerated only where it improves local clarity.

---

## 11. Documentation

- **R-DOC-01** — Every public item has a doc comment. `#![warn(missing_docs)]` in every library crate.
- **R-DOC-02** — Doc comments open with a single-sentence summary in the third person: "Returns the settled total." Not "This function will return...".
- **R-DOC-03** — Document `# Errors` for every `Result`-returning function, `# Panics` for anything that can panic, `# Safety` for every `unsafe fn`.
- **R-DOC-04** — Public API docs include a runnable `# Examples` block (R-TST-21).
- **R-DOC-05** — Comments explain *why*, never *what*. If code needs a comment to explain what it does, rename things until it doesn't.
- **R-DOC-06** — Each crate's `lib.rs` carries a `//!` module doc stating the crate's responsibility and its layer.
- **R-DOC-07** — Architectural decisions are recorded as ADRs in `specs/adr/`, numbered, immutable once accepted, superseded rather than edited.
- **R-DOC-08** — `TODO`/`FIXME` MUST reference a tracked issue: `// TODO(#412): ...`. Untracked TODOs are forbidden.

---

## 12. Dependencies

- **R-DEP-01** — Every new dependency requires justification: maintenance status, licence, transitive weight, and `unsafe` footprint. `std` first; a 30-line helper beats a micro-crate.
- **R-DEP-02** — `Cargo.lock` is committed, for libraries as well as binaries.
- **R-DEP-03** — `cargo deny check` runs in CI: advisory database, licence allow-list, duplicate-version bans.
- **R-DEP-04** — Features are additive and non-breaking. No mutually exclusive features. `default = []` for library crates where practical.
- **R-DEP-05** — Domain-layer dependencies are held to a stricter bar (R-DIR-02): no async runtime, no I/O, no framework.

---

## 13. Tooling gates

- **R-TOOL-01** — The toolchain is pinned in `rust-toolchain.toml` with the exact channel and required components.
- **R-TOOL-02** — Workspace lint configuration lives in `[workspace.lints]` and is inherited by every member. Baseline:
  ```toml
  [workspace.lints.rust]
  unsafe_code = "forbid"
  missing_docs = "warn"
  unreachable_pub = "warn"
  rust_2018_idioms = { level = "warn", priority = -1 }

  [workspace.lints.clippy]
  all = { level = "deny", priority = -1 }
  pedantic = { level = "warn", priority = -1 }
  unwrap_used = "deny"
  expect_used = "deny"
  panic = "deny"
  todo = "deny"
  unimplemented = "deny"
  dbg_macro = "deny"
  print_stdout = "deny"
  print_stderr = "deny"
  await_holding_lock = "deny"
  float_cmp = "deny"
  indexing_slicing = "warn"
  cognitive_complexity = "warn"
  missing_errors_doc = "warn"
  missing_panics_doc = "warn"
  undocumented_unsafe_blocks = "deny"
  ```
- **R-TOOL-03** — Lint suppressions are narrow and justified. `#[allow(...)]` at module or crate scope is forbidden; at item scope it requires a reason:
  ```rust
  #[allow(clippy::too_many_arguments, reason = "generated FFI shim, see ADR-0012")]
  ```
- **R-TOOL-04** — The following MUST pass before any commit is considered complete:
  ```
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo doc --workspace --no-deps          # RUSTDOCFLAGS="-D warnings"
  cargo deny check
  cargo llvm-cov --workspace --fail-under-lines 85
  ```
  Provide this as `cargo xtask ci` so there is exactly one command and no divergence between local and CI.
- **R-TOOL-05** — CI additionally runs `cargo test --release`, `cargo +nightly miri test` (if any `unsafe` exists), and `cargo machete`/`udeps` for unused dependencies.
- **R-TOOL-06** — CI is not advisory. A red gate blocks merge. Never merge with a disabled or skipped gate.

---

## 14. Definition of Done

A unit of work is complete only when **every** box is checked, verified by observed command output rather than assumption (R-AGT-07).

- [ ] A specification exists for the behaviour, and is current (§1)
- [ ] Tests were written first and observed to fail (R-TST-01)
- [ ] Every specified behaviour is covered and traceable to its spec ID (R-SDD-02)
- [ ] Error paths, edge cases, and boundary values are tested (R-TST-17)
- [ ] Contract tests pass for every implementation of every touched port (R-TST-14)
- [ ] Layering holds: `domain` is pure, composition only in `app` (R-DIR-01, R-SEP-02)
- [ ] Every new process-boundary dependency sits behind a port with a fake (R-TRT-01, R-TRT-20)
- [ ] No new trait with a single implementation and no prospect of a second (R-TRT-06)
- [ ] No `unwrap`/`expect`/`panic!`/`dbg!`/`println!` in production paths (R-ERR-02)
- [ ] Domain identifiers are newtypes; invalid states are unrepresentable (§7)
- [ ] Public items documented, with `# Errors` and runnable examples (§11)
- [ ] `cargo xtask ci` passes clean, no new warnings, no new suppressions (R-TOOL-04)
- [ ] Coverage thresholds met (R-TST-16)
- [ ] Deviations recorded with rule IDs; `MUST` deviations have an ADR (§0.2)
- [ ] Spec, ADRs, and glossary updated in the same commit (R-SDD-05)

---

## 15. Amendment

This constitution is version-controlled and amended by pull request with an accompanying ADR stating the problem, the proposed rule change, and the migration path for existing code. Rules are added with new IDs; retired rules are marked `RETIRED` in place. Agents MUST NOT amend this document. The repository-layout amendment recorded in `ADR-002` is a one-time, explicit human-authorized deviation from that prohibition and does not establish a standing exception.
