//! The extra character `ls -l` prints after the permission bits when a file
//! has extended attributes or an access control list.

use std::path::Path;

/// `@` when the file has extended attributes, as BSD `ls` prints on macOS.
/// An access control list (`+`) cannot be read without unsafe code, so it is
/// not reported.
#[cfg(target_os = "macos")]
pub fn marker(path: &Path) -> &'static str {
    match xattr::list(path) {
        Ok(mut names) => {
            if names.next().is_some() {
                "@"
            } else {
                ""
            }
        }
        Err(_) => "",
    }
}

/// `+` when the file has a POSIX ACL and `.` when it has an SELinux context,
/// as GNU `ls` prints on Linux
#[cfg(not(target_os = "macos"))]
pub fn marker(path: &Path) -> &'static str {
    let Ok(names) = xattr::list(path) else {
        return "";
    };
    let (mut acl, mut context) = (false, false);
    for name in names {
        match name.to_str() {
            Some("system.posix_acl_access" | "system.posix_acl_default") => acl = true,
            Some("security.selinux") => context = true,
            _ => {}
        }
    }
    if acl {
        "+"
    } else if context {
        "."
    } else {
        ""
    }
}
