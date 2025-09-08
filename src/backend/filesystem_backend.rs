use crate::parser::ast::{Action, Condition};
use std::fs;
use std::path::PathBuf;

pub struct FileSystemBackend {
    base_dir: PathBuf,
}

impl FileSystemBackend {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        self.base_dir.join(path)
    }

    /// Executes a file system action. Returns true if the action was handled.
    pub fn execute_action(&self, action: &Action) -> bool {
        match action {
            Action::CreateFile { path, content } => {
                fs::write(self.resolve_path(path), content).expect("Failed to create file");
                true
            }
            Action::DeleteFile { path } => {
                let resolved_path = self.resolve_path(path);
                if resolved_path.exists() {
                    fs::remove_file(resolved_path).expect("Failed to delete file");
                }
                true
            }
            Action::CreateDir { path } => {
                let resolved_path = self.resolve_path(path);
                if !resolved_path.exists() {
                    fs::create_dir_all(resolved_path).expect("Failed to create directory");
                }
                true
            }
            Action::DeleteDir { path } => {
                let resolved_path = self.resolve_path(path);
                if resolved_path.exists() {
                    fs::remove_dir_all(resolved_path).expect("Failed to delete directory");
                }
                true
            }
            _ => false, // Ignore actions not meant for this backend
        }
    }

    /// Checks a file system condition.
    pub fn check_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::FileExists { path } => self.resolve_path(path).exists(),
            Condition::FileDoesNotExist { path } => !self.resolve_path(path).exists(),
            Condition::DirExists { path } => self.resolve_path(path).is_dir(),
            Condition::FileContains { path, content } => {
                if let Ok(file_content) = fs::read_to_string(self.resolve_path(path)) {
                    file_content.contains(content)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // --- Condition Checking Methods ---

    pub fn file_exists(&self, path: &str) -> bool {
        let resolved_path = self.resolve_path(path);
        println!("Checking if file exists: {}", resolved_path.display());
        resolved_path.exists()
    }

    pub fn file_does_not_exist(&self, path: &str) -> bool {
        let resolved_path = self.resolve_path(path);
        println!(
            "Checking if file does not exist: {}",
            resolved_path.display()
        );
        !resolved_path.exists()
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        let resolved_path = self.resolve_path(path);
        println!("Checking if dir exists: {}", resolved_path.display());
        resolved_path.is_dir()
    }

    pub fn file_contains(&self, path: &str, content: &str) -> bool {
        if let Ok(file_content) = fs::read_to_string(self.resolve_path(path)) {
            println!("Checking if file contains: {}", content);
            file_content.contains(content)
        } else {
            false
        }
    }
}
