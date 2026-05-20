mod utils;

use crate::utils::{push_flag, push_joined, push_option};
use derive_builder::Builder;
use std::ffi::OsString;
use std::process::{Command, ExitStatus};
use tap::Tap;

#[derive(Builder, Clone, Debug, Default)]
#[builder(pattern = "owned", default)]
pub struct CargoInstall {
    root: Option<std::path::PathBuf>,
    version: Option<OsString>,
    git: Option<OsString>,
    branch: Option<OsString>,
    tag: Option<OsString>,
    rev: Option<OsString>,
    path: Option<std::path::PathBuf>,
    force: bool,
    locked: bool,
    debug: bool,
    features: Vec<OsString>,
    all_features: bool,
    no_default_features: bool,
    extra_args: Vec<OsString>,
}

impl CargoInstall {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn args(&self) -> Vec<OsString> {
        vec![OsString::from("install")].tap_mut(|args| {
            push_option(args, "--root", self.root.as_deref());
            push_option(args, "--version", self.version.as_deref());
            push_option(args, "--git", self.git.as_deref());
            push_option(args, "--branch", self.branch.as_deref());
            push_option(args, "--tag", self.tag.as_deref());
            push_option(args, "--rev", self.rev.as_deref());
            push_option(args, "--path", self.path.as_deref());
            push_flag(args, "--force", self.force);
            push_flag(args, "--locked", self.locked);
            push_flag(args, "--debug", self.debug);
            push_joined(args, "--features", &self.features, ",");
            push_flag(args, "--all-features", self.all_features);
            push_flag(args, "--no-default-features", self.no_default_features);
            args.extend(self.extra_args.iter().cloned());
        })
    }

    pub fn command(&self) -> Command {
        Command::new("cargo").tap_mut(|command| {
            command.args(self.args());
        })
    }

    pub fn run(&self) -> std::io::Result<ExitStatus> {
        self.command().status()
    }
}

#[cfg(test)]
mod tests;
