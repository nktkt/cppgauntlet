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
- prebuilt macOS and Linux binaries attached to GitHub releases
- Homebrew tap support

Until those release channels exist, use `cargo install --git` or `cargo install --path`.
