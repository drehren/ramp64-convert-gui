use std::sync::mpsc::{Receiver, Sender};

use super::check_can_add_file;

pub(super) enum Work {
  ScanDirectory(std::path::PathBuf),
}

pub(super) enum WorkResult {
  ScanDirectory(ScanDirResult),
}

pub(super) type ScanDirResult = Result<Vec<std::path::PathBuf>, GenericError>;

#[derive(Debug)]
pub(super) struct GenericError {
  pub path: std::path::PathBuf,
  pub error: std::io::Error,
}

impl GenericError {
  fn new(error: std::io::Error, path: std::path::PathBuf) -> Self {
    Self { path, error }
  }
}

impl std::fmt::Display for GenericError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!(
      "Error {}: {}",
      self.path.display(),
      self.error
    ))
  }
}

impl std::error::Error for GenericError {}

pub(super) fn start_worker_thread(receiver: Receiver<Work>, result_sender: Sender<WorkResult>) {
  std::thread::spawn(move || {
    for work in receiver.iter() {
      let _ = result_sender.send(match work {
        Work::ScanDirectory(dir) => WorkResult::ScanDirectory(scan_directory(dir)),
      });
    }
  });
}

fn scan_directory(dir: std::path::PathBuf) -> ScanDirResult {
  let mut files = Vec::new();
  for entry in (std::fs::read_dir(&dir).map_err(|e| GenericError::new(e, dir))?).flatten() {
    let path = entry.path();
    if check_can_add_file(&path) {
      files.push(path)
    }
  }

  Ok(files)
}
