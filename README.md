# cargo-install

Wrapper around the `cargo install` command

## Quick Start

```rust,no_run
use cargo_install::CargoInstallBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    CargoInstallBuilder::default()
        .crate_name("ripgrep")
        .version("14.1.1")
        .bin("rg")
        .profile("release")
        .locked(true)
        .build()?
        .run()?;

    Ok(())
}
```

This builds and runs a command equivalent to:

```text
cargo install --version 14.1.1 --bin rg --profile release --locked ripgrep
```
