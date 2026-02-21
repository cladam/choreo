use crate::colours;
use crate::parser::ast::Action;
use chrono::Utc;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use sysinfo::System;
use uuid as rust_uuid;

/// SystemBackend handles System actor actions and conditions.
/// This includes logging, pausing, timestamps, UUIDs, and service/port checks.
pub struct SystemBackend {
    /// Stores output from system actions for test verification
    pub last_output: String,
}

impl SystemBackend {
    pub fn new() -> Self {
        Self {
            last_output: String::new(),
        }
    }

    /// Executes a System action. Returns true if the action was handled.
    pub fn execute_action(
        &mut self,
        action: &Action,
        env_vars: &mut HashMap<String, String>,
        verbose: bool,
    ) -> bool {
        match action {
            // System log: surface the message into output and log it.
            Action::Log { message } => {
                colours::info(&format!("[SYSTEM] {}", message));
                if !self.last_output.is_empty() && !self.last_output.ends_with('\n') {
                    self.last_output.push('\n');
                }
                self.last_output.push_str(&format!("System: {}\n", message));
                true
            }

            // Pause: sleep for the specified duration (seconds).
            Action::Pause { duration } => {
                let dur = Duration::from_secs_f32(*duration);
                thread::sleep(dur);
                true
            }

            // Timestamp: set a variable to the current timestamp.
            Action::Timestamp { variable } => {
                let now = Utc::now();
                let ts = now.format("%Y-%m-%d_%H:%M:%S").to_string();
                env_vars.insert(variable.clone(), ts.clone());
                if verbose {
                    colours::info(&format!("[SYSTEM] Set {} = {}", variable, ts));
                }
                if !self.last_output.is_empty() && !self.last_output.ends_with('\n') {
                    self.last_output.push('\n');
                }
                self.last_output
                    .push_str(&format!("Timestamp {} = {}\n", variable, ts));
                true
            }

            // Uuid: set a variable to a generated v4 UUID.
            Action::Uuid { variable } => {
                let uid = rust_uuid::Uuid::new_v4().to_string();
                env_vars.insert(variable.clone(), uid.clone());
                if verbose {
                    colours::info(&format!("[SYSTEM] Set {} = {}", variable, uid));
                }
                if !self.last_output.is_empty() && !self.last_output.ends_with('\n') {
                    self.last_output.push('\n');
                }
                self.last_output
                    .push_str(&format!("Uuid {} = {}\n", variable, uid));
                true
            }

            _ => false, // Not a system action
        }
    }

    /// Clears the last output buffer.
    pub fn clear_output(&mut self) {
        self.last_output.clear();
    }

    // --- System Condition Checks ---

    /// Checks if a service/process is running using cross-platform sysinfo crate.
    pub fn check_service_is_running(&self, name: &str, verbose: bool) -> bool {
        if verbose {
            println!("[SYSTEM] Checking if service/process '{}' is running", name);
        }

        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        // Check if any process matches the name (case-insensitive on Windows)
        let name_lower = name.to_lowercase();
        for process in sys.processes().values() {
            let process_name = process.name().to_string_lossy().to_lowercase();
            // Match exact name or name without extension (for Windows .exe)
            if process_name == name_lower
                || process_name == format!("{}.exe", name_lower)
                || process_name.trim_end_matches(".exe") == name_lower
            {
                if verbose {
                    println!(
                        "[SYSTEM] Found process '{}' with PID {}",
                        process.name().to_string_lossy(),
                        process.pid()
                    );
                }
                return true;
            }
        }

        false
    }

    /// Checks if a service is stopped (not running).
    pub fn check_service_is_stopped(&self, name: &str, verbose: bool) -> bool {
        if verbose {
            println!("[SYSTEM] Checking if service '{}' is stopped", name);
        }
        !self.check_service_is_running(name, false)
    }

    /// Checks if a service/executable is installed on the system (cross-platform).
    pub fn check_service_is_installed(&self, name: &str, verbose: bool) -> bool {
        if verbose {
            println!(
                "[SYSTEM] Checking if service/executable '{}' is installed",
                name
            );
        }

        // Use the `which` crate for cross-platform executable lookup
        if which::which(name).is_ok() {
            if verbose {
                println!("[SYSTEM] Found '{}' in PATH", name);
            }
            return true;
        }

        // Platform-specific additional checks for service definitions
        #[cfg(target_os = "macos")]
        {
            // Check launchd plist files
            let launchd_paths = [
                format!("/Library/LaunchDaemons/{}.plist", name),
                format!("/Library/LaunchAgents/{}.plist", name),
                format!(
                    "{}/Library/LaunchAgents/{}.plist",
                    std::env::var("HOME").unwrap_or_default(),
                    name
                ),
            ];

            for path in &launchd_paths {
                if std::path::Path::new(path).exists() {
                    if verbose {
                        println!("[SYSTEM] Found launchd plist at {}", path);
                    }
                    return true;
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check systemd unit files
            let systemd_paths = [
                format!("/etc/systemd/system/{}.service", name),
                format!("/lib/systemd/system/{}.service", name),
                format!("/usr/lib/systemd/system/{}.service", name),
            ];

            for path in &systemd_paths {
                if std::path::Path::new(path).exists() {
                    if verbose {
                        println!("[SYSTEM] Found systemd unit at {}", path);
                    }
                    return true;
                }
            }

            // Check init.d
            let initd_path = format!("/etc/init.d/{}", name);
            if std::path::Path::new(&initd_path).exists() {
                if verbose {
                    println!("[SYSTEM] Found init.d script at {}", initd_path);
                }
                return true;
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Check Windows services using sc query
            let output = std::process::Command::new("sc")
                .args(["query", name])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    if verbose {
                        println!("[SYSTEM] Found Windows service '{}'", name);
                    }
                    return true;
                }
            }
        }

        false
    }

    /// Checks if a port is currently listening for connections.
    pub fn check_port_is_listening(&self, port: u16, verbose: bool) -> bool {
        if verbose {
            println!("[SYSTEM] Checking if port {} is listening", port);
        }

        use std::net::TcpListener;

        // Try to bind to the port - if it fails with AddrInUse, the port is in use (listening)
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(_) => {
                // We were able to bind, so no one is listening on this port
                false
            }
            Err(e) => {
                // If the error is "address already in use", then something is listening
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    true
                } else {
                    // Some other error (e.g., permission denied for low ports)
                    // Fall back to checking with lsof or netstat
                    self.check_port_with_system_command(port, verbose)
                }
            }
        }
    }

    /// Checks if a port is closed (not listening).
    pub fn check_port_is_closed(&self, port: u16, verbose: bool) -> bool {
        if verbose {
            println!("[SYSTEM] Checking if port {} is closed", port);
        }
        !self.check_port_is_listening(port, false)
    }

    /// Helper function to check port using system commands (fallback).
    fn check_port_with_system_command(&self, port: u16, verbose: bool) -> bool {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("lsof")
                .args(["-i", &format!(":{}", port), "-P", "-n"])
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("[SYSTEM] lsof output: {}", stdout);
                }
                return stdout.contains("LISTEN");
            }
            false
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("ss")
                .args(["-tlnp", &format!("sport = :{}", port)])
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("[SYSTEM] ss output: {}", stdout);
                }
                // ss returns header + data lines if port is listening
                return stdout.lines().count() > 1;
            }
            false
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("netstat").args(["-an"]).output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("[SYSTEM] netstat output (checking for port {})", port);
                }
                // Look for LISTENING state on the specified port
                let port_pattern = format!(":{}", port);
                return stdout.lines().any(|line| {
                    line.contains(&port_pattern) && line.to_uppercase().contains("LISTENING")
                });
            }
            false
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            if verbose {
                println!("[SYSTEM] Port check not fully supported on this platform");
            }
            false
        }
    }
}

impl Default for SystemBackend {
    fn default() -> Self {
        Self::new()
    }
}
