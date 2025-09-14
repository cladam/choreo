use crate::parser::ast::{Action, TestSuiteSettings};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use terminal_size::{terminal_size, Height, Width};

pub struct TerminalBackend {
    // For interactive PTY sessions (`types`, `presses`)
    pty_writer: Box<dyn Write + Send>,
    pty_output_receiver: Receiver<String>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    #[allow(dead_code)]
    reader_thread: Option<JoinHandle<()>>,
    // For non-interactive command execution (`runs`)
    pub last_stdout: String,
    pub last_stderr: String,
    cwd: PathBuf,
}

impl TerminalBackend {
    /// Creates a new backend with a PTY session.
    /// - `base_dir`: The directory where the shell process should start.
    /// - `shell_path`: An optional path to a specific shell executable.
    pub fn new(base_dir: PathBuf, settings: TestSuiteSettings) -> Self {
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

        // Use the provided shell path, or default to "zsh".
        let shell = settings.shell_path.unwrap();
        // Spawn the shell process.
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(base_dir.clone());
        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("Failed to spawn command");

        // We need to get a reader and writer for the PTY's master end.
        let reader = pair
            .master
            .try_clone_reader()
            .expect("Failed to clone reader");
        let writer = pair.master.take_writer().expect("Failed to take writer");

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
            pty_writer: writer,
            pty_output_receiver: receiver,
            child,
            reader_thread: Some(reader_thread),
            last_stdout: String::new(),
            last_stderr: String::new(),
            cwd: base_dir,
        }
    }

    /// Reads from the interactive PTY buffer. This is for `types` and `presses`.
    pub fn read_pty_output(&mut self, pty_buffer: &mut String) {
        for new_output in self.pty_output_receiver.try_iter() {
            pty_buffer.push_str(&new_output);
        }

        // Append the stdout from the last non-interactive `run` command, if any.
        if !self.last_stdout.is_empty() {
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
    ) -> bool {
        match action {
            Action::Type { content, .. } => {
                self.pty_writer.write_all(content.as_bytes()).unwrap();
                self.pty_writer.flush().unwrap();
                true
            }
            Action::Press { key, .. } if key == "Enter" => {
                self.pty_writer.write_all(b"\n").unwrap();
                self.pty_writer.flush().unwrap();
                true
            }
            Action::Run { command, .. } => {
                // Special handling for 'cd' to update the backend's CWD.
                if command.trim().starts_with("cd ") {
                    let path_str = command.trim().strip_prefix("cd ").unwrap().trim();
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
                // This is the new, robust implementation.
                *last_exit_code = None;
                self.last_stdout.clear();
                self.last_stderr.clear();

                let mut child = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&self.cwd)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to spawn command");

                if let Some(t) = timeout {
                    // This is a crude way to poll for completion with a timeout.
                    // A more robust solution might use `wait_timeout`.
                    let start = std::time::Instant::now();
                    while start.elapsed() < t {
                        if let Ok(Some(status)) = child.try_wait() {
                            let output = child.wait_with_output().expect("Failed to get output");
                            *last_exit_code = status.code();
                            self.last_stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            self.last_stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            return true;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    // If we get here, the process timed out.
                    child.kill().expect("Failed to kill timed-out process");
                    *last_exit_code = Some(137); // Convention for timeout
                    self.last_stderr = "Command timed out".to_string();
                } else {
                    // No timeout, wait indefinitely.
                    let output = child.wait_with_output().expect("Failed to execute command");
                    *last_exit_code = output.status.code();
                    self.last_stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    self.last_stderr = String::from_utf8_lossy(&output.stderr).to_string();
                }
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
