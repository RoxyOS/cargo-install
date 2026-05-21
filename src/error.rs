use crate::utils::{extract_backticked_after, extract_backticked_before};
use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error)]
/// Errors returned when executing `cargo install`.
pub enum CargoInstallError {
    /// The `cargo` executable could not be found in `PATH`.
    #[error("`cargo` command is not installed or not found in PATH")]
    CargoNotInstalled,
    /// Spawning or waiting on the `cargo` process failed for an I/O reason.
    #[error("failed to execute `cargo install`: {0}")]
    Io(#[from] std::io::Error),
    /// Cargo reported that the package is already installed.
    ///
    /// `stderr` contains the full original cargo output.
    #[error("package is already installed: {package}")]
    AlreadyInstalled { package: String, stderr: String },
    /// Cargo reported that a binary with the same name already exists.
    ///
    /// `stderr` contains the full original cargo output.
    #[error("binary already exists in destination: {binary}")]
    BinaryAlreadyExists { binary: String, stderr: String },
    /// Cargo reported that the selected package has no installable binaries or examples.
    ///
    /// `stderr` contains the full original cargo output.
    #[error("package has no installable binaries or examples")]
    NoInstallableTargets { stderr: String },
    /// Cargo failed while compiling the selected package.
    ///
    /// `stderr` contains the full original cargo output.
    #[error("failed to compile package: {package}")]
    CompileFailed { package: String, stderr: String },
    /// Cargo exited with a non-success status that this crate did not recognize.
    ///
    /// `stderr` contains the full original cargo output.
    #[error("unknown `cargo install` error with status {status}: {stderr}")]
    UnknownCargoError { status: ExitStatus, stderr: String },
}

impl CargoInstallError {
    pub(crate) fn from_spawn_error(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::CargoNotInstalled
        } else {
            Self::Io(error)
        }
    }

    pub(crate) fn from_output(status: ExitStatus, stderr: Vec<u8>) -> Self {
        let stderr = String::from_utf8_lossy(&stderr).trim().to_owned();

        if let Some(package) = extract_backticked_before(&stderr, "is already installed") {
            return Self::AlreadyInstalled { package, stderr };
        }

        if let Some(binary) = extract_backticked_after(&stderr, "binary `") {
            return Self::BinaryAlreadyExists { binary, stderr };
        }

        if stderr.contains("specified package has no binaries")
            || stderr.contains("no packages found with binaries or examples")
        {
            return Self::NoInstallableTargets { stderr };
        }

        if let Some(package) = extract_backticked_after(&stderr, "failed to compile `") {
            return Self::CompileFailed { package, stderr };
        }

        Self::UnknownCargoError { status, stderr }
    }
}
