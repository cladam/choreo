use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "choreo",
    author = "Claes Adamsson @cladam",
    version,
    about = "choreo: A test runner for CLI tools, BDD-style",
    long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a choreography test suite.
    Run {
        /// Path to the choreography test suite file.
        #[arg(short, long, default_value = "test.chor")]
        file: String,
        /// Enable verbose output for debugging.
        #[arg(long)]
        verbose: bool,
    },
    /// Update choreo to the latest version.
    #[command(name = "update", hide = true)] // Hidden from help
    Update,
}
