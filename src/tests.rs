use crate::CargoInstallBuilder;
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
fn execution_uses_fake_cargo_and_preserves_exit_status() {
    let _guard = env_lock().lock().unwrap();
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(temp.path());
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

    let status = install.run().unwrap();

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
fn missing_program_returns_io_error() {
    let _guard = env_lock().lock().unwrap();
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

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn free_function_forwards_raw_args() {
    let _guard = env_lock().lock().unwrap();
    let temp = tempdir().unwrap();
    let script_path = fake_cargo_script(temp.path());
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

    let status = CargoInstallBuilder::default()
        .extra_args(vec![
            "ripgrep".into(),
            "--locked".into(),
            output_path.as_os_str().into(),
        ])
        .build()
        .unwrap()
        .run()
        .unwrap();

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
            "install\nripgrep\n--locked\n{}\n",
            temp.path().join("captured-args.txt").display()
        )
    );
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(unix)]
fn fake_cargo_script(dir: &Path) -> PathBuf {
    let script_path = dir.join("cargo");
    fs::write(
        &script_path,
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
exit 23
"#,
    )
    .unwrap();

    let mut permissions = fs::metadata(&script_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).unwrap();
    script_path
}
