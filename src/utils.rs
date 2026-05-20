use std::ffi::{OsStr, OsString};

pub(crate) fn push_option<T>(args: &mut Vec<OsString>, flag: &str, value: Option<&T>)
where
    T: AsRef<OsStr> + ?Sized,
{
    if let Some(value) = value {
        args.push(flag.into());
        args.push(value.as_ref().into());
    }
}

pub(crate) fn push_flag(args: &mut Vec<OsString>, flag: &str, enabled: bool) {
    if enabled {
        args.push(flag.into());
    }
}

pub(crate) fn push_joined(
    args: &mut Vec<OsString>,
    flag: &str,
    values: &[OsString],
    separator: &str,
) {
    if !values.is_empty() {
        args.push(flag.into());
        args.push(join_os_strings(values, separator));
    }
}

pub(crate) fn join_os_strings(parts: &[OsString], separator: &str) -> OsString {
    let mut joined = OsString::new();

    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            joined.push(separator);
        }
        joined.push(part);
    }

    joined
}
