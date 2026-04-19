use std::path::{Path, PathBuf};

pub fn fixture_path(relative_path: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}
