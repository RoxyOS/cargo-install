//! Wrapper around the `cargo install` command.
//!
//! The crate exposes a builder for the most common `cargo install`
//! options, plus `extra_args` for unsupported flags.
//!
//! # Example
//!
//! ```rust,no_run
//! use cargo_install::CargoInstallBuilder;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     CargoInstallBuilder::default()
//!         .crate_name("ripgrep")
//!         .version("14.1.1")
//!         .bin("rg")
//!         .profile("release")
//!         .locked(true)
//!         .build()?
//!         .run()?;
//!
//!     Ok(())
//! }
//! ```

mod error;
mod utils;

pub use crate::error::CargoInstallError;

use crate::utils::{push_flag, push_joined, push_option};
use derive_builder::Builder;
use std::ffi::OsString;
use std::process::{Command, Stdio};
use tap::Tap;

#[derive(Builder, Debug, Default)]
#[builder(pattern = "owned", default, setter(into, strip_option))]
/// Configuration for a `cargo install` invocation.
///
/// Construct this type directly with [`CargoInstall::new`] or prefer the
/// generated `CargoInstallBuilder` for a more ergonomic setup flow.
pub struct CargoInstall {
    /// Sets `--root` to override the installation root directory.
    root: Option<std::path::PathBuf>,
    /// Sets `--version` to install a specific crate version requirement.
    version: Option<OsString>,
    /// Sets `--git` to install from a git repository.
    git: Option<OsString>,
    /// Sets `--branch` when installing from git.
    branch: Option<OsString>,
    /// Sets `--tag` when installing from git.
    tag: Option<OsString>,
    /// Sets `--rev` when installing from git.
    rev: Option<OsString>,
    /// Sets `--target` to build for a specific compilation target.
    target: Option<OsString>,
    /// Sets `--bin` to install a specific binary target.
    bin: Option<OsString>,
    /// Sets `--profile` to select the build profile used for installation.
    profile: Option<OsString>,
    /// Sets `--path` to install from a local crate directory.
    path: Option<std::path::PathBuf>,
    /// Sets the registry crate name to install.
    crate_name: Option<OsString>,
    /// Enables `--force`.
    force: bool,
    /// Enables `--locked`.
    locked: bool,
    /// Enables `--debug`.
    debug: bool,
    /// Sets `--features` using a feature list.
    features: Vec<OsString>,
    /// Enables `--all-features`.
    all_features: bool,
    /// Enables `--no-default-features`.
    no_default_features: bool,
    /// Appends additional arguments after all typed options.
    extra_args: Vec<OsString>,
    /// Overrides the child process stdout configuration.
    ///
    /// When not set, stdout inherits from the current process.
    stdout: Option<Stdio>,
}

impl CargoInstall {
    /// Creates an empty `CargoInstall` configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the `cargo install` argument vector in canonical flag order.
    ///
    /// The returned list always starts with `install`, followed by typed
    /// options and flags, then `crate_name`, followed by `extra_args`.
    pub fn args(&self) -> Vec<OsString> {
        vec![OsString::from("install")].tap_mut(|args| {
            push_option(args, "--root", self.root.as_deref());
            push_option(args, "--version", self.version.as_deref());
            push_option(args, "--git", self.git.as_deref());
            push_option(args, "--branch", self.branch.as_deref());
            push_option(args, "--tag", self.tag.as_deref());
            push_option(args, "--rev", self.rev.as_deref());
            push_option(args, "--target", self.target.as_deref());
            push_option(args, "--bin", self.bin.as_deref());
            push_option(args, "--profile", self.profile.as_deref());
            push_option(args, "--path", self.path.as_deref());
            push_flag(args, "--force", self.force);
            push_flag(args, "--locked", self.locked);
            push_flag(args, "--debug", self.debug);
            push_joined(args, "--features", &self.features, ",");
            push_flag(args, "--all-features", self.all_features);
            push_flag(args, "--no-default-features", self.no_default_features);
            if let Some(crate_name) = self.crate_name.as_ref() {
                args.push(crate_name.clone());
            }
            args.extend(self.extra_args.iter().cloned());
        })
    }

    fn command(mut self) -> Command {
        Command::new("cargo").tap_mut(|command| {
            command.args(self.args());
            command.stdout(self.stdout.take().unwrap_or_else(Stdio::inherit));
            command.stderr(Stdio::piped());
        })
    }

    /// Executes `cargo install` and maps common stderr patterns into
    /// [`CargoInstallError`].
    ///
    /// `stdout` inherits from the current process unless overridden.
    /// `stderr` is always captured so the crate can parse known failure modes.
    ///
    /// Error classification depends on the stderr text produced by the local
    /// cargo version, so unrecognized messages fall back to
    /// [`CargoInstallError::UnknownCargoError`].
    pub fn run(self) -> Result<(), CargoInstallError> {
        let output = self
            .command()
            .spawn()
            .map_err(CargoInstallError::from_spawn_error)?
            .wait_with_output()?;
        let status = output.status;

        if status.success() {
            Ok(())
        } else {
            Err(CargoInstallError::from_output(status, output.stderr))
        }
    }
}

#[cfg(test)]
mod tests;
