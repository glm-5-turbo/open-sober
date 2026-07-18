// SPDX-License-Identifier: MIT
//
// Directory and path setup helpers.

use std::path::PathBuf;

/// Get the cache directory for Open Sober.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("open-sober")
}

/// Get the directory for extracted native libraries.
pub fn libs_dir() -> PathBuf {
    cache_dir().join("libs")
}