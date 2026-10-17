# Scripts

Small helpers so you do not have to remember the `cargo` invocations. Every
script comes as PowerShell (`.ps1`, for this Windows checkout) and POSIX shell
(`.sh`, for Linux/macOS or Git Bash). They all `cd` to the repository root first,
so you can call them from anywhere.

## Usage

| Task | PowerShell | shell |
| --- | --- | --- |
| Debug build | `.\scripts\build.ps1` | `scripts/build.sh` |
| Release build | `.\scripts\release.ps1` | `scripts/release.sh` |
| Run (debug) | `.\scripts\run.ps1` | `scripts/run.sh` |
| Run (release) | `.\scripts\run-release.ps1` | `scripts/run-release.sh` |
| Tests | `.\scripts\test.ps1` | `scripts/test.sh` |
| CI gates (fmt + clippy + test) | `.\scripts\check.ps1` | `scripts/check.sh` |
| Clean | `.\scripts\clean.ps1` | `scripts/clean.sh` |
| Benchmarks | `.\scripts\bench.ps1` | `scripts/bench.sh` |

## Opening a folder straight away

The app takes an optional folder argument:

```powershell
.\scripts\run-release.ps1 H:\OneDrive\Pictures
```

```sh
scripts/run-release.sh ~/Pictures
```

Without it the app starts with an empty folder box: type a path and press
**Open**.

For a large library prefer `run-release`: a debug `cargo run` spends noticeable
time resizing images. The first release build takes a few minutes; later runs
are instant.

## Benchmarks

`bench` runs the analysis harnesses in `crates/pinacoteca-bench`, which are never
shipped. It measures the decoders and the mip pyramid:

```powershell
.\scripts\bench.ps1 H:\OneDrive\Pictures
.\scripts\bench.ps1 --limit 50 H:\OneDrive\Pictures
.\scripts\bench.ps1 thumbs H:\OneDrive\Pictures   # thumbnail pipeline
```

## A note on the toolchain

`rust-toolchain.toml` pins Rust 1.98.0 and CI installs the same version, so
`check` here and CI agree exactly. If `rustup` shows a different default, run
`rustup show` in the repository root: it selects the pinned toolchain
automatically.
