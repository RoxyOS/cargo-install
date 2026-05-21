# cargo-install

Wrapper around the `cargo install` command

## Quick Start

```rust,no_run
use cargo_install::CargoInstallBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    CargoInstallBuilder::default()
        .crate_name(Some("ripgrep".into()))
        .version(Some("14.1.1".into()))
        .locked(true)
        .build()?
        .run()?;

    Ok(())
}
```

This builds and runs a command equivalent to:

```text
cargo install --version 14.1.1 --locked ripgrep
```
