use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
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
}

impl TerminalBackend {
    pub fn new() -> Self {
        // Get the size of the user's actual terminal.
        let term_size = terminal_size();
        let (cols, rows) = if let Some((Width(w), Height(h))) = term_size {
            (w, h)
        } else {
            // Provide a sensible default if the size can't be determined.
            (100, 40)
        };
        // 1. Create a new PtySystem.
        let pty_system = NativePtySystem::default();

        // 2. Create a PTY pair.
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to open pty");

        // 3. Spawn the command in the PTY.
        let cmd = CommandBuilder::new("zsh");
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
        }
    }

    /// Checks for new output from the reader thread without blocking.
    pub fn read_output(&mut self, output_buffer: &mut String) {
        for new_output in self.output_receiver.try_iter() {
            print!("{}", new_output); // Echo to the real console for debugging
            output_buffer.push_str(&new_output);
        }
    }

    /// Executes a single action from the AST.
    pub fn execute_action(&mut self, action: &crate::ast::Action) {
        match action {
            crate::ast::Action::Type { content, .. } => {
                self.writer.write_all(content.as_bytes()).unwrap();
                self.writer.flush().unwrap();
            }
            crate::ast::Action::Press { key, .. } if key == "Enter" => {
                self.writer.write_all(b"\n").unwrap();
            }
            // ... handle other actions
            _ => {}
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
