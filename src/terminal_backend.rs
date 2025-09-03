use crate::ast::{Action, Condition};
use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use portable_pty::PtyPair;

pub struct TerminalBackend {
    // The underlying pseudo-terminal process.
    pty_pair: PtyPair,

    // The receiving end of the channel to get output from the reader thread.
    output_receiver: Receiver<String>,

    // A handle to the reader thread, so we can manage its lifecycle.
    // We wrap it in an Option to allow us to take ownership of it for a clean shutdown.
    #[allow(dead_code)]
    reader_thread: Option<JoinHandle<()>>,
}