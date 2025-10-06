use crate::parser::ast::Action;
use std::collections::HashMap;
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
            // Check if the path starts with the same directory as cwd
            if let Some(cwd_name) = cwd.file_name() {
                if let Some(cwd_str) = cwd_name.to_str() {
                    if path.starts_with(&format!("{}/", cwd_str)) {
                        // If path already includes the cwd directory, resolve from parent
                        if let Some(parent) = cwd.parent() {
                            return parent.join(p);
                        }
                    }
                }
            }
            cwd.join(p)
        }
    }

    /// Executes a file system action. Returns true if the action was handled.
    pub fn execute_action(
        &self,
        action: &Action,
        cwd: &Path,
        env_vars: &mut HashMap<String, String>,
    ) -> bool {
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
            Action::ReadFile { path, variable } => {
                let resolved_path = self.resolve_path(path, cwd);
                match fs::read_to_string(&resolved_path) {
                    Ok(content) => {
                        env_vars.insert(variable.clone().unwrap().to_string(), content);
                        true
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to read file {} (resolved to {:?}): {}",
                            path, resolved_path, e
                        );
                        false
                    }
                }
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
        println!(
            "Checking if file does not exist: {}",
            resolved_path.display()
        );
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
