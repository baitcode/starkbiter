#![warn(missing_docs)]

//! `Arbiter` CLI Tool
//!
//! The Arbiter command-line interface provides minimum utilities for the
//! utilization of the arbiter-core crate. It is designed to be a simple and
//! versatile.
//!
//!
//! Key Features:
//! - Simulation Initialization: Allow users to kickstart new data analysis
//!   simulations.
//! - Contract Bindings: Generate necessary bindings for interfacing with
//!   different contracts.
//!
//!
//! This CLI leverages the power of Rust's type system to
//! offer fast and reliable operations, ensuring data integrity and ease of use.

use std::{env, fs, path::Path};

use clap::{command, CommandFactory, Parser, Subcommand};
use config::{Config, ConfigError};
use serde::Deserialize;
use thiserror::Error;

mod bind;

/// Represents command-line arguments passed to the `Arbiter` tool.
#[derive(Parser)]
#[command(name = "Arbiter")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Starknet Virtual Machine Logic Simulator", long_about = None)]
#[command(author)]
struct Args {
    /// Defines the subcommand to execute.
    #[command(subcommand)]
    command: Option<Commands>,
}

/// `ConfigurationError` enumeration type for errors parsing a `.toml`
/// configuration file.
#[derive(Error, Debug)]
pub enum ArbiterError {
    /// Indicates an error occurred during the parsing of the configuration
    /// file.
    #[error("Error with config parsing: {0}")]
    ConfigError(#[from] config::ConfigError),
    /// Indicates that the configuration file could not be read from the given
    /// path.
    #[error("Error with file IO: {0}")]
    IOError(#[from] std::io::Error),

    /// Indicates an error occurred during the deserialization of the `.toml`
    /// file.
    #[error("Error with toml deserialization: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Indicates an error occurred during processing of a JSON file.
    #[error("Error with serde_json: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Indicates an error occurred with a database.
    #[error("Error with DB: {0}")]
    DBError(String),
}

/// Defines available subcommands for the `Arbiter` tool.
#[derive(Subcommand)]
enum Commands {
    /// Reads compiled Sierra contract classes and generates rust bindings.
    Bind {
        /// The path to the directory with contracts or a specific contract file
        #[clap(index = 1)]
        contract_class_path: String,
        /// The path to the output directory for generated bindings.
        #[clap(index = 2)]
        output_dir: String,
        /// Add Debug to #[derive] macross.
        #[clap(long)]
        use_debug: bool,
    },
    // Does nothing really as forking is real time.
    Fork {
        /// The name of the config file used to configure the fork.
        #[clap(index = 1)]
        fork_config_path: String,
        #[clap(long)]
        overwrite: bool,
    },
}

/// The main entry point for the `Arbiter` tool.
///
/// This function parses command line arguments, and based on the provided
/// subcommand, either initializes a new simulation or generates bindings.
///
/// # Returns
///
/// * A `Result` which is either an empty tuple for successful execution or a
///   dynamic error.
fn main() -> Result<(), ArbiterError> {
    let args = Args::parse();

    match &args.command {
        Some(Commands::Bind {
            contract_class_path,
            output_dir,
            use_debug,
        }) => {
            println!("Generating bindings from JSON...");
            bind::cainome_bind(contract_class_path, output_dir, use_debug)?;
        }
        Some(Commands::Fork {
            fork_config_path,
            overwrite,
        }) => {
            println!("Not Forking...");
        }
        None => Args::command().print_long_help()?,
    }

    Ok(())
}
