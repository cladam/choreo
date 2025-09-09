use crate::parser::ast::Action;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::{env, fs, thread};
use terminal_size::{terminal_size, Height, Width};

pub struct TerminalBackend {
    // The writer part of the pseudo-terminal.
    writer: Box<dyn Write + Send>,

    // The receiving end of the channel to get output from the reader thread.
    output_receiver: Receiver<String>,

    // A handle to the child process, used to terminate it.
    child: Box<dyn portable_pty::Child + Send + Sync>,

    // A handle to the reader thread.
    #[allow(dead_code)]
    reader_thread: Option<JoinHandle<()>>,

    base_dir: PathBuf,
}

impl TerminalBackend {
    pub fn new(base_dir: PathBuf) -> Self {
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

        // Spawn the command in the PTY.
        let mut cmd = CommandBuilder::new("zsh");
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

        // 4. Create the channel for communication.
        let (sender, receiver): (Sender<String>, Receiver<String>) = mpsc::channel();

        // 5. Spawn the reader thread.
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
            writer,
            output_receiver: receiver,
            child,
            reader_thread: Some(reader_thread),
            base_dir,
        }
    }

    /// Reads output and checks for special exit code markers.
    pub fn read_output(&mut self, output_buffer: &mut String, last_exit_code: &mut Option<i32>) {
        for new_output in self.output_receiver.try_iter() {
            output_buffer.push_str(&new_output);
        }

        // Check for our special exit code line.
        // Instead of parsing terminal output, read the exit code from a temp file.
        let exit_code_file = env::temp_dir().join("choreo_exit_code.tmp");
        if exit_code_file.exists() {
            if let Ok(code_str) = fs::read_to_string(&exit_code_file) {
                println!("{}", code_str);
                if let Ok(code) = code_str.trim().parse::<i32>() {
                    *last_exit_code = Some(code);
                }
                // Clean up the file after reading.
                fs::remove_file(exit_code_file).ok();
            }
        }
    }

    /// Executes a single action from the AST. Returns true if the action was handled.
    pub fn execute_action(
        &mut self,
        action: &crate::parser::ast::Action,
        last_exit_code: &mut Option<i32>,
    ) -> bool {
        match action {
            Action::Type { content, .. } => {
                self.writer.write_all(content.as_bytes()).unwrap();
                self.writer.flush().unwrap();
                true
            }
            Action::Press { key, .. } if key == "Enter" => {
                self.writer.write_all(b"\n").unwrap();
                true
            }
            Action::Run { command, .. } => {
                // Reset the last exit code before running a new command.
                *last_exit_code = None;

                // Define a temporary file to store the exit code.
                let exit_code_file = env::temp_dir().join("choreo_exit_code.tmp");

                // Use `sh -c` to execute the command and then write the exit code to the temp file.
                // This is more robust than parsing PTY output.
                let escaped_command = command.replace('\'', "'\\''");
                let full_command = format!(
                    "sh -c '{}; echo $? > {}'\n",
                    escaped_command,
                    exit_code_file.to_str().unwrap()
                );
                println!("{}", full_command);

                self.writer.write_all(full_command.as_bytes()).unwrap();
                self.writer.flush().unwrap();
                true
            }
            _ => false, // Ignore actions not meant for this backend
        }
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        // Kill the child process when the backend is no longer in use.
        self.child.kill().ok();
        //println!("\nTerminalBackend dropped and child process terminated.");
    }
}
