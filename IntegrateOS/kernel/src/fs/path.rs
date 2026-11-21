//! Canonical Path Normalization & Decomposition
//!
//! Provides path parsing, component extraction, and standard `.` / `..` resolution.

use alloc::string::String;
use alloc::vec::Vec;

/// Normalizes a path string into a canonical absolute path (e.g. `"/a/b/../c/./d"` -> `"/a/c/d"`).
pub fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return String::from("/");
    }

    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        } else if seg == ".." {
            parts.pop();
        } else {
            parts.push(seg);
        }
    }

    if parts.is_empty() {
        return String::from("/");
    }

    let mut normalized = String::new();
    for seg in parts {
        normalized.push('/');
        normalized.push_str(seg);
    }
    normalized
}

/// Splits a canonical path into `(parent_path, child_name)`.
///
/// For example:
/// - `"/home/user/file.txt"` -> `("/home/user", "file.txt")`
/// - `"/file.txt"` -> `("/", "file.txt")`
/// - `"/"` -> `("/", "")`
pub fn split_path(canonical_path: &str) -> (&str, &str) {
    if canonical_path == "/" || canonical_path.is_empty() {
        return ("/", "");
    }

    let trimmed = canonical_path.trim_end_matches('/');
    if let Some(idx) = trimmed.rfind('/') {
        if idx == 0 {
            ("/", &trimmed[1..])
        } else {
            (&trimmed[..idx], &trimmed[idx + 1..])
        }
    } else {
        ("/", trimmed)
    }
}
