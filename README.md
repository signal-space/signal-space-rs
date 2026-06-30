# signal-space-rs

Rust types and validation for Signal Space.

Supported `signal-space-spec`: `0.1.0`

The crate provides serde models for the shared surface contract, fixture
validation, and an optional `lazily-runtime` feature for integration with
`lazily-rs`.

## Runtime Feature

Enable `lazily-runtime` to represent a `SignalGraph` as a lazily writable cell
with derived inspector summaries and read-only Snapshot/Delta mirror exports:

```bash
cargo test --features lazily-runtime
```

The runtime keeps writable graph state separate from deterministic derived
projections. Product adapters still own mutation authority; exported
Snapshot/Delta data is a read-only mirror for UI and tooling consumers.
