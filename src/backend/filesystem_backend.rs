use crate::parser::ast::Action;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileSystemBackend {}

impl FileSystemBackend {
    pub fn new() -> Self {
        Self {}
    }

    pub(crate) fn resolve_path(&self, path: &str, cwd: &Path) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    }

    /// Executes a file system action. Returns true if the action was handled.
    pub fn execute_action(&self, action: &Action, cwd: &Path) -> bool {
        match action {
            Action::CreateFile { path, content } => {
                fs::write(self.resolve_path(path, cwd), content).expect("Failed to create file");
                true
            }
            Action::DeleteFile { path } => {
                let resolved_path = self.resolve_path(path, cwd);
                if resolved_path.exists() {
                    fs::remove_file(resolved_path).expect("Failed to delete file");
                }
                true
            }
            Action::CreateDir { path } => {
                let resolved_path = self.resolve_path(path, cwd);
                if !resolved_path.exists() {
                    fs::create_dir_all(resolved_path).expect("Failed to create directory");
                }
                true
            }
            Action::DeleteDir { path } => {
                let resolved_path = self.resolve_path(path, cwd);
                println!("Deleting directory: {}", resolved_path.display());
                if resolved_path.exists() {
                    fs::remove_dir_all(resolved_path).expect("Failed to delete directory");
                }
                true
            }
            _ => false, // Ignore actions not meant for this backend
        }
    }

    // --- Condition Checking Methods ---

    pub fn file_exists(&self, path: &str, cwd: &Path, verbose: bool) -> bool {
        let resolved_path = self.resolve_path(path, cwd);
        if verbose {
            println!("Checking if file exists: {}", resolved_path.display());
        }
        resolved_path.exists()
    }

    pub fn file_does_not_exist(&self, path: &str, cwd: &Path, verbose: bool) -> bool {
        let resolved_path = self.resolve_path(path, cwd);
        if verbose {
            println!(
                "Checking if file does not exist: {}",
                resolved_path.display()
            );
        }
        !resolved_path.exists()
    }

    pub fn dir_exists(&self, path: &str, cwd: &Path, verbose: bool) -> bool {
        let resolved_path = self.resolve_path(path, cwd);
        if verbose {
            println!("Checking if dir exists: {}", resolved_path.display());
        }
        resolved_path.is_dir()
    }

    pub fn file_contains(&self, path: &str, content: &str, cwd: &Path, verbose: bool) -> bool {
        if let Ok(file_content) = fs::read_to_string(self.resolve_path(path, cwd)) {
            if verbose {
                println!("File content: {}", file_content);
            }
            file_content.contains(content)
        } else {
            false
        }
    }

    pub fn dir_does_not_exist(&self, path: &str, cwd: &Path, verbose: bool) -> bool {
        let resolved_path = self.resolve_path(path, cwd);
        if verbose {
            println!(
                "Checking if dir does not exist: {}",
                resolved_path.display()
            );
        }
        !resolved_path.exists()
    }
}
