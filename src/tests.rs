use crate::{CargoInstallBuilder, CargoInstallError};
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
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
fn command_applies_configured_stdio() {
    let install = CargoInstallBuilder::default()
        .stdout(Some(Stdio::null()))
        .build()
        .unwrap();

    let command = install.command();

    assert_eq!(command.get_program(), OsStr::new("cargo"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        vec![OsStr::new("install")]
    );
}

#[test]
fn run_returns_cargo_not_installed_when_cargo_is_missing() {
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

    assert!(matches!(error, CargoInstallError::CargoNotInstalled));
}

#[test]
fn run_parses_compile_failed_from_real_cargo_install() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let crate_dir = write_failing_crate(temp.path(), "compile-fail");
    let root = temp.path().join("root");

    let error = CargoInstallBuilder::default()
        .path(Some(crate_dir))
        .root(Some(root))
        .build()
        .unwrap()
        .run()
        .unwrap_err();

    assert!(matches!(
        error,
        CargoInstallError::CompileFailed { ref package, .. } if package.starts_with("compile-fail v0.1.0")
    ));
}

#[test]
fn run_succeeds_on_real_cargo_install() {
    let _guard = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = tempdir().unwrap();
    let crate_dir = write_binary_crate(temp.path(), "real-install-ok", "real-install-ok");
    let root = temp.path().join("root");

    let result = CargoInstallBuilder::default()
        .path(Some(crate_dir))
        .root(Some(root.clone()))
        .build()
        .unwrap()
        .run();

    assert!(result.is_ok());
    assert!(installed_binary_path(&root, "real-install-ok").exists());
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
        "error: binary `shared-bin` already exists in destination as part of `first-pkg v0.1.0`\nAdd --force to overwrite\n",
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
        CargoInstallError::BinaryAlreadyExists { ref binary, .. } if binary == "shared-bin"
    ));
}

#[test]
fn run_falls_back_to_unknown_error_for_unrecognized_stderr() {
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
            if status.code() == Some(55) && stderr.contains("error: something unrecognized happened")
    ));
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn write_binary_crate(base: &Path, package_name: &str, bin_name: &str) -> PathBuf {
    let crate_dir = base.join(package_name);
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "{bin_name}"
path = "src/main.rs"
"#
        ),
    )
    .unwrap();
    fs::write(
        src_dir.join("main.rs"),
        format!("fn main() {{ println!(\"{}\"); }}\n", package_name),
    )
    .unwrap();
    crate_dir
}

fn write_failing_crate(base: &Path, package_name: &str) -> PathBuf {
    let crate_dir = base.join(package_name);
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"
"#
        ),
    )
    .unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() { let _ = ; }\n").unwrap();
    crate_dir
}

fn installed_binary_path(root: &Path, binary_name: &str) -> PathBuf {
    root.join("bin").join(binary_name)
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
case "$last" in
    /*)
        printf '%s\n' "$output" > "$last"
        ;;
esac
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
