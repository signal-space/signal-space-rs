# signal-space-rs

Rust types and validation for Signal Space.

Supported `signal-space-spec`: `0.3.0` (also validates `0.2.0` documents)

The crate provides serde models for the shared surface contract, fixture
validation, and an optional `lazily-runtime` feature for integration with
`lazily-rs`.

`0.3.0` adds optional typed ports (`PortSpec`), live cable telemetry
(`StreamTelemetry`), and external transport bindings (`IoBinding`) plus edge
`from_port`/`to_port`. Every addition is optional, so `0.2.0` documents still
parse and validate.

## Runtime Feature

Enable `lazily-runtime` to represent a `SignalGraph` as a lazily writable source
with derived inspector summaries and read-only Snapshot/Delta mirror exports:

```bash
cargo test --features lazily-runtime
```

The runtime keeps writable graph state separate from deterministic derived
projections. Product adapters still own mutation authority; exported
Snapshot/Delta data is a read-only mirror for UI and tooling consumers.
