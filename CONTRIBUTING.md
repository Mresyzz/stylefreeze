# Contributing to StyleFreeze

Thank you for contributing to StyleFreeze.

## Development Setup

Requirements:
- Rust 1.85+ / latest stable
- Node.js 20+ (for npm wrapper testing)
- Git

Clone and run tests:
```bash
git clone https://github.com/Mresyzz/stylefreeze.git
cd stylefreeze
cargo test --all
```

Run micro-benchmarks:
```bash
cargo bench
```

## Adding a New Language Grammar

StyleFreeze uses `tree-sitter` parsers. To add support for a new language:
1. Add the corresponding `tree-sitter-<language>` crate to `Cargo.toml`.
2. Add an enum variant to `SupportedLang` in `src/lang.rs`.
3. Configure string and comment token rules in `src/ast.rs`.
4. Add unit tests in `src/engine.rs` asserting AST normalization and whitespace preservation.
5. Submit a pull request.

## Code Guidelines
- Keep execution latency sub-5ms. Avoid heap allocations inside hunk traversal loops.
- Maintain documentation integrity and zero external runtime dependencies.
- Ensure all tests pass with `cargo test --all`.
