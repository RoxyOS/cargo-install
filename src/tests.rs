use crate::{CargoInstallBuilder, CargoInstallError};
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;

#[test]
fn empty_builder_produces_cargo_install() {
    let install = CargoInstallBuilder::default().build().unwrap();
    let command = install.command();

    assert_eq!(command.get_program(), OsStr::new("cargo"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        vec![OsStr::new("install")]
    );
}

#[test]
fn derived_builder_constructs_install() {
    let install = CargoInstallBuilder::default()
        .version(Some("1.2.3".into()))
        .force(true)
        .extra_args(vec!["ripgrep".into()])
        .build()
        .unwrap();

    let command = install.command();
    let args = command.get_args().collect::<Vec<_>>();

    assert_eq!(
        args,
        vec![
            OsStr::new("install"),
            OsStr::new("--version"),
            OsStr::new("1.2.3"),
            OsStr::new("--force"),
            OsStr::new("ripgrep"),
        ]
    );
}

#[test]
fn typed_flags_render_in_canonical_order() {
    let install = CargoInstallBuilder::default()
        .root(Some("/tmp/root".into()))
        .version(Some("1.2.3".into()))
        .git(Some("https://example.com/repo.git".into()))
        .branch(Some("stable".into()))
        .tag(Some("v1.2.3".into()))
        .rev(Some("abc123".into()))
        .path(Some("vendor/pkg".into()))
        .force(true)
        .locked(true)
        .debug(true)
        .features(vec!["cli".into(), "tls".into()])
        .all_features(true)
        .no_default_features(true)
        .build()
        .unwrap();

    let command = install.command();
    let args = command.get_args().collect::<Vec<_>>();

    assert_eq!(
        args,
        vec![
            OsStr::new("install"),
            OsStr::new("--root"),
            OsStr::new("/tmp/root"),
            OsStr::new("--version"),
            OsStr::new("1.2.3"),
            OsStr::new("--git"),
            OsStr::new("https://example.com/repo.git"),
            OsStr::new("--branch"),
            OsStr::new("stable"),
            OsStr::new("--tag"),
            OsStr::new("v1.2.3"),
            OsStr::new("--rev"),
            OsStr::new("abc123"),
            OsStr::new("--path"),
            OsStr::new("vendor/pkg"),
            OsStr::new("--force"),
            OsStr::new("--locked"),
            OsStr::new("--debug"),
            OsStr::new("--features"),
            OsStr::new("cli,tls"),
            OsStr::new("--all-features"),
            OsStr::new("--no-default-features"),
        ]
    );
}

#[test]
fn raw_args_are_appended_after_typed_options() {
    let install = CargoInstallBuilder::default()
        .version(Some("1.0.0".into()))
        .extra_args(vec!["ripgrep".into(), "--force".into(), "--quiet".into()])
        .build()
        .unwrap();

    let command = install.command();
    let args = command.get_args().collect::<Vec<_>>();

    assert_eq!(
        args,
        vec![
            OsStr::new("install"),
            OsStr::new("--version"),
            OsStr::new("1.0.0"),
            OsStr::new("ripgrep"),
            OsStr::new("--force"),
            OsStr::new("--quiet"),
        ]
    );
}

#[test]
fn command_status_returns_fake_cargo_exit_status() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(temp.path(), 23, "");
    let output_path = temp.path().join("captured-args.txt");
    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let install = CargoInstallBuilder::default()
        .version(Some("1.0.0".into()))
        .extra_args(vec![
            "ripgrep".into(),
            "--locked".into(),
            output_path.as_os_str().into(),
        ])
        .build()
        .unwrap();

    let status = install.command().status().unwrap();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert_eq!(status.code(), Some(23));
    let captured = fs::read_to_string(output_path).unwrap();
    assert_eq!(
        captured,
        format!(
            "install\n--version\n1.0.0\nripgrep\n--locked\n{}\n",
            temp.path().join("captured-args.txt").display()
        )
    );
}

#[test]
fn run_returns_io_error_when_cargo_is_missing() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let original_path = std::env::var_os("PATH");

    unsafe {
        std::env::set_var("PATH", "");
    }

    let install = CargoInstallBuilder::default().build().unwrap();
    let error = install.run().unwrap_err();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(matches!(
        error,
        CargoInstallError::CargoNotInstalled
    ));
}

#[test]
fn run_returns_non_zero_exit_error() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(
        temp.path(),
        23,
        "error: failed to compile `ripgrep v14.1.1`, intermediate artifacts can be found at `/tmp/cargo-installabc`\n",
    );
    let output_path = temp.path().join("captured-args.txt");

    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let error = CargoInstallBuilder::default()
        .extra_args(vec![
            "ripgrep".into(),
            "--locked".into(),
            output_path.as_os_str().into(),
        ])
        .build()
        .unwrap()
        .run()
        .unwrap_err();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(matches!(
        error,
        CargoInstallError::CompileFailed { ref package, .. } if package == "ripgrep v14.1.1"
    ));
    let captured = fs::read_to_string(output_path).unwrap();
    assert_eq!(
        captured,
        format!(
            "install\nripgrep\n--locked\n{}\n",
            temp.path().join("captured-args.txt").display()
        )
    );
}

#[test]
fn run_succeeds_on_zero_exit_status() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(temp.path(), 0, "");
    let output_path = temp.path().join("captured-args.txt");
    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let result = CargoInstallBuilder::default()
        .extra_args(vec!["ripgrep".into(), output_path.as_os_str().into()])
        .build()
        .unwrap()
        .run();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(result.is_ok());
    let captured = fs::read_to_string(output_path).unwrap();
    assert_eq!(
        captured,
        format!("install\nripgrep\n{}\n", temp.path().join("captured-args.txt").display())
    );
}

#[test]
fn run_parses_already_installed_error() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(
        temp.path(),
        101,
        "Ignored package `ripgrep v14.1.1` is already installed, use --force to override\n",
    );
    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let error = CargoInstallBuilder::default().build().unwrap().run().unwrap_err();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(matches!(
        error,
        CargoInstallError::AlreadyInstalled { ref package, .. } if package == "ripgrep v14.1.1"
    ));
}

#[test]
fn run_parses_binary_already_exists_error() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(
        temp.path(),
        101,
        "error: binary `cargo-embed` already exists in destination as part of `probe-rs-tools v0.24.0`\nAdd --force to overwrite\n",
    );
    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let error = CargoInstallBuilder::default().build().unwrap().run().unwrap_err();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(matches!(
        error,
        CargoInstallError::BinaryAlreadyExists { ref binary, .. } if binary == "cargo-embed"
    ));
}

#[test]
fn run_falls_back_to_generic_cargo_failure() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(temp.path(), 55, "error: something unrecognized happened\n");
    let original_path = std::env::var_os("PATH");
    let script_dir = script_path.parent().unwrap();
    let mut new_path = std::ffi::OsString::from(script_dir.as_os_str());
    if let Some(existing) = &original_path {
        new_path.push(if cfg!(windows) { ";" } else { ":" });
        new_path.push(existing);
    }

    unsafe {
        std::env::set_var("PATH", &new_path);
    }

    let error = CargoInstallBuilder::default().build().unwrap().run().unwrap_err();

    if let Some(existing) = original_path {
        unsafe {
            std::env::set_var("PATH", existing);
        }
    }

    assert!(matches!(
        error,
        CargoInstallError::UnknownCargoError { status, ref stderr }
            if status.code() == Some(55) && stderr == "error: something unrecognized happened"
    ));
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(unix)]
fn fake_cargo_script(dir: &Path, exit_code: i32, stderr: &str) -> PathBuf {
    let script_path = dir.join("cargo");
    fs::write(
        &script_path,
        format!(
            r#"#!/bin/sh
output=""
for arg in "$@"; do
    if [ "$output" = "" ]; then
        output="$arg"
    else
        output="$output
$arg"
    fi
done
last=""
for arg in "$@"; do
    last="$arg"
done
printf '%s\n' "$output" > "$last"
cat >&2 <<'EOF'
{stderr}EOF
exit {exit_code}
"#,
            stderr = stderr,
        ),
    )
    .unwrap();

    let mut permissions = fs::metadata(&script_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).unwrap();
    script_path
}
