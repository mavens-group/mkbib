// src/core/file_ops.rs
use std::fs;
use std::path::{Path, PathBuf};

pub fn create_backup(file_path: &Path) -> std::io::Result<PathBuf> {
  let mut backup_path = file_path.to_path_buf();

  // Strategy: Append ".bak" to the filename
  // "my_paper.bib" -> "my_paper.bib.bak"
  if let Some(extension) = backup_path.extension() {
    let mut ext_str = extension.to_os_string();
    ext_str.push(".bak");
    backup_path.set_extension(ext_str);
  } else {
    backup_path.set_extension("bak");
  }

  fs::copy(file_path, &backup_path)?;
  Ok(backup_path)
}
