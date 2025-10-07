use crate::colours;
use crate::parser::ast::{Action, TestSuiteSettings};
use crate::parser::helpers::substitute_variables_in_action;
use chrono::Utc;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use terminal_size::{terminal_size, Height, Width};
use uuid as rust_uuid;

pub struct TerminalBackend {
    pty_output_receiver: Receiver<String>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    #[allow(dead_code)]
    reader_thread: Option<JoinHandle<()>>,
    // For non-interactive command execution (`runs`)
    pub last_stdout: String,
    pub last_stderr: String,
    cwd: PathBuf,
    settings: TestSuiteSettings,
}

impl TerminalBackend {
    /// Creates a new backend with a PTY session.
    /// - `base_dir`: The directory where the shell process should start.
    /// - `shell_path`: An optional path to a specific shell executable.
    pub fn new(cwd: PathBuf, settings: TestSuiteSettings) -> Self {
        // Get the size of the user's actual terminal.
        let term_size = terminal_size();
        let (cols, rows) = if let Some((Width(w), Height(h))) = term_size {
            (w, h)
        } else {
            // Provide a sensible default if the size can't be determined.
            (100, 40)
        };
        // Create a new PtySystem.
        let pty_system = NativePtySystem::default();

        // Create a PTY pair.
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to open pty");

        let shell_path = settings
            .shell_path
            .clone()
            .unwrap_or_else(|| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(shell_path);
        cmd.cwd(&cwd);
        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("Failed to spawn command");

        // We need to get a reader for the PTY's master end.
        let reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone reader");

        // Create the channel for communication.
        let (sender, receiver): (Sender<String>, Receiver<String>) = mpsc::channel();

        // Spawn the reader thread.
        let reader_thread = thread::spawn(move || {
            // This thread will block here, but it won't freeze the main program.
            for byte in reader.bytes() {
                let byte = byte.expect("PTY reader failed");
                let char_str = String::from_utf8_lossy(&[byte]).to_string();

                // Only send non-empty strings through the channel.
                if !char_str.is_empty() {
                    if sender.send(char_str).is_err() {
                        break;
                    }
                }
            }
        });

        Self {
            pty_output_receiver: receiver,
            child,
            reader_thread: Some(reader_thread),
            last_stdout: String::new(),
            last_stderr: String::new(),
            cwd,
            settings,
        }
    }

    /// Reads from the interactive PTY buffer. This is for `types` and `presses`.
    pub fn read_pty_output(&mut self, pty_buffer: &mut String) {
        for new_output in self.pty_output_receiver.try_iter() {
            pty_buffer.push_str(&new_output);
        }

        // Append the stdout from the last non-interactive `run` command, if any.
        if !self.last_stdout.is_empty() {
            // Ensure there's a newline before appending the output from `run`.
            if !pty_buffer.ends_with('\n') && !pty_buffer.is_empty() {
                pty_buffer.push('\n');
            }
            pty_buffer.push_str(&self.last_stdout);
            // Clear it so it's not appended again on the next check.
            self.last_stdout.clear();
        }
    }

    /// Executes a single action from the AST. Returns true if the action was handled.
    pub fn execute_action(
        &mut self,
        action: &Action,
        last_exit_code: &mut Option<i32>,
        timeout: Option<Duration>,
        _env_vars: &mut HashMap<String, String>,
    ) -> bool {
        let action = substitute_variables_in_action(action, _env_vars);
        match action {
            Action::Run { command, .. } => {
                // Special handling for 'cd' to update the backend's CWD.
                let mut choreo_command = command.clone();
                let trimmed = choreo_command.trim();
                if trimmed.starts_with("cd ") {
                    let path_str = trimmed.strip_prefix("cd ").unwrap().trim();
                    let new_path = self.cwd.join(path_str);
                    if new_path.is_dir() {
                        self.cwd = new_path.canonicalize().unwrap_or_else(|_| new_path.clone());
                        *last_exit_code = Some(0);
                        self.last_stdout.clear();
                        self.last_stderr.clear();
                    } else {
                        *last_exit_code = Some(1);
                        self.last_stdout.clear();
                        self.last_stderr = format!("cd: no such file or directory: {}", path_str);
                    }
                    return true;
                }

                // Detect trailing & (allow whitespace before it)
                if trimmed.ends_with('&') {
                    // Remove the trailing ampersand and any extra whitespace
                    let without_amp = trimmed[..trimmed.rfind('&').unwrap_or(trimmed.len())]
                        .trim_end()
                        .to_string();

                    // Build a safe nohup wrapper to fully detach the process.
                    // Escape is intentionally minimal: the original command is assumed to be a shell snippet.
                    choreo_command = format!("nohup {} >/dev/null 2>&1 &", without_amp);

                    colours::info(&format!(
                        "[TERMINAL] Spawning detached background command: {}",
                        without_amp
                    ));
                }

                // Reset last command results
                *last_exit_code = None;
                self.last_stdout.clear();
                self.last_stderr.clear();

                let shell = self.settings.shell_path.as_deref().unwrap_or("/bin/sh");
                let mut child = Command::new(shell)
                    .arg("-c")
                    .arg(choreo_command)
                    .current_dir(&self.cwd)
                    .stdin(Stdio::null()) // Prevent hanging on commands waiting for stdin
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to spawn command");

                let mut stdout_handle = child.stdout.take().unwrap();
                let mut stderr_handle = child.stderr.take().unwrap();

                let stdout_thread = thread::spawn(move || {
                    let mut buf = Vec::new();
                    stdout_handle
                        .read_to_end(&mut buf)
                        .expect("Failed to read stdout");
                    buf
                });

                let stderr_thread = thread::spawn(move || {
                    let mut buf = Vec::new();
                    stderr_handle
                        .read_to_end(&mut buf)
                        .expect("Failed to read stderr");
                    buf
                });

                let status = if let Some(t) = timeout {
                    // This is a crude way to poll for completion with a timeout.
                    // A more robust solution might use `wait_timeout`.
                    let start = std::time::Instant::now();
                    let mut status = None;
                    while start.elapsed() < t {
                        match child.try_wait() {
                            Ok(Some(s)) => {
                                status = Some(s);
                                break;
                            }
                            Ok(None) => {
                                thread::sleep(Duration::from_millis(50));
                                continue;
                            }
                            Err(e) => panic!("Error attempting to wait for child: {}", e),
                        }
                    }

                    if status.is_none() {
                        // If we get here, the process timed out.
                        child.kill().expect("Failed to kill timed-out process");
                        self.last_stderr = "Command timed out".to_string();
                    }
                    status
                } else {
                    // No timeout, wait indefinitely.
                    Some(child.wait().expect("Failed to wait on child"))
                };

                let stdout_bytes = stdout_thread.join().unwrap();
                let stderr_bytes = stderr_thread.join().unwrap();

                self.last_stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
                self.last_stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
                *last_exit_code = status.and_then(|s| s.code()).or_else(|| {
                    if self.last_stderr == "Command timed out" {
                        Some(137)
                    } else {
                        None
                    }
                });

                true
            }

            // System log: surface the message into the interactive output and log it.
            Action::Log { message } => {
                colours::info(&format!("[SYSTEM] {}", message));
                if !self.last_stdout.is_empty() && !self.last_stdout.ends_with('\n') {
                    self.last_stdout.push('\n');
                }
                self.last_stdout.push_str(&format!("System: {}\n", message));
                true
            }

            // Pause: sleep for the specified duration (seconds).
            Action::Pause { duration } => {
                // Expecting `duration` as a floating-point number of seconds.
                let dur = Duration::from_secs_f32(duration);
                thread::sleep(dur);
                true
            }

            // Timestamp: set a variable to the current timestamp (seconds.nanos).
            Action::Timestamp { variable } => {
                let now = Utc::now();
                let ts = now.format("%Y-%m-%d_%H:%M:%S").to_string();
                _env_vars.insert(variable.clone(), ts.clone());
                if !self.last_stdout.is_empty() && !self.last_stdout.ends_with('\n') {
                    self.last_stdout.push('\n');
                }
                self.last_stdout
                    .push_str(&format!("Timestamp {} = {}\n", variable, ts));
                true
            }

            // Uuid: set a variable to a generated v4 UUID.
            Action::Uuid { variable } => {
                let uid = rust_uuid::Uuid::new_v4().to_string();
                _env_vars.insert(variable.clone(), uid.clone());
                if !self.last_stdout.is_empty() && !self.last_stdout.ends_with('\n') {
                    self.last_stdout.push('\n');
                }
                self.last_stdout
                    .push_str(&format!("Uuid {} = {}\n", variable, uid));
                true
            }

            _ => false, // Ignore actions not meant for this backend
        }
    }

    /// Returns the current working directory of the terminal backend.
    pub fn get_cwd(&self) -> &Path {
        &self.cwd
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        // Terminate the child process.
        if let Err(e) = self.child.kill() {
            eprintln!("Failed to kill child process: {}", e);
        }
        // Wait for the child process to exit.
        let _ = self.child.wait();
    }
}
