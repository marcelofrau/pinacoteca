# Contributing

Thanks for your interest in Pinacoteca.

## Before you start

Read these first, in order:

1. [docs/vision.md](docs/vision.md) — what the project is and is not.
2. [docs/scope.md](docs/scope.md) — what is in and out of scope.
3. [docs/cleanroom-protocol.md](docs/cleanroom-protocol.md) — **mandatory** rules.

The clean-room rules are a hard requirement. A pull request that violates them
(for example, one that copies code, assets or design from the reference
application, or that was written while consulting its disassembly) cannot be
merged.

## Development

Requirements:

- A recent stable Rust toolchain (see `rust-version` in `Cargo.toml`).

Build and run:

```sh
cargo build --release
cargo run -p pinacoteca
```

## Checks

Every change must pass the same checks that CI runs:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Pull requests

- Keep changes focused; one logical change per pull request.
- Add or update tests for behavior changes.
- Update documentation when behavior or scope changes. Scope changes require an
  edit to [docs/scope.md](docs/scope.md).
- Reference the specification a change implements where one exists under
  `docs/analysis/`.
- Never commit anything from `binaries/` or `analysis-scratch/`.

## Commit messages

Use a clear, conventional style, for example:

```
feat(viewer): add fit-to-window zoom
fix(grid): keep selection when thumbnails load
docs(scope): move RAW decoding to post-M5 candidates
```

## Reporting issues

Describe the observed behavior, the expected behavior, and the exact steps to
reproduce. If the issue concerns a reference-application behavior, describe only
the observable behavior — do not attach disassembly or proprietary assets.
