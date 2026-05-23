# Installation

CppGauntlet is currently distributed from source. The CLI is implemented in Rust and shells out to C++ tooling for verification workflows.

## Requirements

Required:

- Rust stable toolchain
- `clang++` for compile, sanitizer, coverage, and fuzzing workflows

Optional tools unlock additional workflows:

- `cmake` and `ctest` for CMake project checks
- `clang-tidy` for static analysis
- `llvm-cov` and `llvm-profdata` for source-based coverage
- libFuzzer support through a Clang build that accepts `-fsanitize=fuzzer`

Check your environment with:

```bash
cppgauntlet doctor
```

## Install From GitHub

```bash
cargo install --git https://github.com/nktkt/cppgauntlet
```

Verify the install:

```bash
cppgauntlet --version
cppgauntlet doctor
```

## Install From GitHub Releases

Tagged releases publish macOS and Linux archives from the `Release` workflow:

```bash
tar -xzf cppgauntlet-v0.1.0-linux-x86_64.tar.gz
./cppgauntlet-v0.1.0-linux-x86_64/cppgauntlet --version
```

Verify the archive first with the matching `.sha256` file:

```bash
shasum -a 256 -c cppgauntlet-v0.1.0-linux-x86_64.tar.gz.sha256
```

Tagged release assets also include GitHub artifact attestations. Verify provenance with:

```bash
gh attestation verify cppgauntlet-v0.1.0-linux-x86_64.tar.gz \
  --repo nktkt/cppgauntlet \
  --signer-workflow nktkt/cppgauntlet/.github/workflows/release.yml
```

## Install From a Local Checkout

```bash
git clone https://github.com/nktkt/cppgauntlet.git
cd cppgauntlet
cargo install --path .
```

For development, run from the checkout without installing:

```bash
cargo run -- check tests/fixtures/hello.cpp --sanitizers none
```

## Future Distribution

Planned distribution targets include:

- crates.io with `cargo install cppgauntlet`
- Homebrew tap support

Until crates.io and Homebrew release channels exist, use `cargo install --git`, GitHub release archives, or `cargo install --path`.
