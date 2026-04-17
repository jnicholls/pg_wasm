# Agent instructions

This file orients automated assistants and humans working in this repository.

## Rust coding standards

Apply these rules to **all Rust sources** (`*.rs`) in this workspace.

### Edition, language, and standard library

- Prefer **Rust 2024 edition** capabilities and idioms when they match existing project style.
- Prefer features and APIs from the **latest Rust toolchain** used by the project; treat the standard library as authoritative at https://doc.rust-lang.org/std/.

### `#[derive(...)]` attribute order

- List every `#[derive(...)]` proc-macro attribute in **strict alphabetical order** (e.g. `Clone` before `Debug` before `Eq`).

### `use` import layout

- Split `use` lines into **exactly three sections**, in this order, with **one blank line** between sections:
  1. **Standard library** (`std`, `core`, `alloc`, etc.).
  2. **External crates** (dependencies from crates.io or git).
  3. **Project internals** (`crate::...`, `super::...`, `self::...`).
- Inside each section, group imports by **top-level crate or module** and use **brace lists** `{}` when pulling multiple items from the same path.

### `Option` and `Result` handling

- **Do not** call `unwrap()` on `Option` or `Result` except inside **tests** (unit tests, integration tests, `#[cfg(test)]` modules).
- **Avoid** `expect()` when a **better** pattern exists for the situation, such as propagating with `?`, enriching errors (`map_err`, `context`), or an explicit `match` / `if let` that preserves intent.

### Import depth and symbol paths

- **Types**: do not spell out long paths at every use site (for example `crate::module1::module2::MyType`). Import `MyType` (or its parent module, per local style) at the top of the file or module.
- **Functions and constants**: do not call through long paths like `crate::module1::module2::function()`. Import the **leaf module** you need (for example `use crate::module1::module2`) and call **`module2::function()`** so references stay shallow (typically **two path segments** after the import).

When in doubt, match patterns already used in neighboring modules in this repository.
