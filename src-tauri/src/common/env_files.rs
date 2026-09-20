//! Load development `.env` files using OS-native paths.
//!
//! `tauri dev` often uses `src-tauri/` as the working directory. Packaged
//! binaries use the executable directory. Search both without Unix-only paths.

use std::env;
use std::path::PathBuf;

/// Candidate `.env` locations, first existing file wins for new variables.
///
/// `dotenvy` does not override variables already present in the process.
pub fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        push_unique(&mut paths, cwd.join(".env"));
        if let Some(parent) = cwd.parent() {
            push_unique(&mut paths, parent.join(".env"));
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            push_unique(&mut paths, dir.join(".env"));
            if let Some(parent) = dir.parent() {
                push_unique(&mut paths, parent.join(".env"));
            }
        }
    }

    paths
}

pub fn load() {
    for path in candidates() {
        if path.is_file() {
            let _ = dotenvy::from_path(&path);
        }
    }
    let _ = dotenvy::dotenv();
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::candidates;
    use std::ffi::OsStr;

    #[test]
    fn candidates_use_env_filename_on_all_platforms() {
        let paths = candidates();
        assert!(
            !paths.is_empty(),
            "at least cwd or executable dir should yield a candidate"
        );
        for path in &paths {
            assert_eq!(path.file_name(), Some(OsStr::new(".env")));
        }
    }
}
