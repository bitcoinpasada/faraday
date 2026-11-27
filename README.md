# Faraday, the `next` branch

`next` is the rebuild of Faraday by layer: the domain and the port, the UI kit, the application,
the runtime, the adapters, the roots and the tools, each a set of crates in one Cargo workspace.
Its design is `docs/spec/`; the crate list and the manifest of every crate is
`docs/spec/09-manifest.md`.

## Repository layout

| Path | What it holds |
|---|---|
| `core/` | the domain and the port: every crate of `layer = "domain"` and `layer = "port"` |
| `kit/` | the UI kit, tokens to templates |
| `app/` | the application: the kernels, parts, features, composites, owners and product crates |
| `runtime/` | the runtime |
| `adapters/` | the adapters and the two adapter processes |
| `roots/` | the root binaries and the tour root |
| `tools/` | the tool and test-support crates, the lints, the cross-check, references, vectors and the scripts `just` runs |
| `build/` | the image build and the build list |
| `fuzz/` | the fuzz targets |
| `third_party/` | vendored crates, each with its patches |
| `supply-chain/` | cargo vet's audits and exemptions |
| `docs/` | what is true now or still to be built |
| `.github/`, `.config/`, `.claude/` | as on `main` |
| files | `.gitignore`, `CLAUDE.md`, `CONTRIBUTING.md`, `Cargo.lock`, `Cargo.toml`, `LICENSE`, `README.md`, `SECURITY.md`, `deny.toml`, `justfile`, `rust-toolchain.toml` |
