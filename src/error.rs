use crate::utils::{extract_backticked_after, extract_backticked_before};
use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CargoInstallError {
    #[error("`cargo` command is not installed or not found in PATH")]
    CargoNotInstalled,
    #[error("failed to execute `cargo install`: {0}")]
    Io(#[from] std::io::Error),
    #[error("package is already installed: {package}")]
    AlreadyInstalled { package: String, stderr: String },
    #[error("binary already exists in destination: {binary}")]
    BinaryAlreadyExists { binary: String, stderr: String },
    #[error("package has no installable binaries or examples")]
    NoInstallableTargets { stderr: String },
    #[error("failed to compile package: {package}")]
    CompileFailed { package: String, stderr: String },
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
