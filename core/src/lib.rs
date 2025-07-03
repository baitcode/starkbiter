//! ```text
//! ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
//! ░░      ░░░        ░░░      ░░░  ░░░░  ░░       ░░░        ░░        ░░        ░░       ░░
//! ▒  ▒▒▒▒▒▒▒▒▒▒▒  ▒▒▒▒▒  ▒▒▒▒  ▒▒  ▒▒▒  ▒▒▒  ▒▒▒▒  ▒▒▒▒▒  ▒▒▒▒▒▒▒▒  ▒▒▒▒▒  ▒▒▒▒▒▒▒▒  ▒▒▒▒  ▒
//! ▓▓      ▓▓▓▓▓▓  ▓▓▓▓▓  ▓▓▓▓  ▓▓     ▓▓▓▓▓       ▓▓▓▓▓▓  ▓▓▓▓▓▓▓▓  ▓▓▓▓▓      ▓▓▓▓       ▓▓
//! ███████  █████  █████        ██  ███  ███  ████  █████  ████████  █████  ████████  ███  ██
//! ██      ██████  █████  ████  ██  ████  ██       ███        █████  █████        ██  ████  █
//! ██████████████████████████████████████████████████████████████████████████████████████████
//! ```
//!                                              
//! `starkbiter-core` is designed to facilitate agent-based simulations over the
//! Starknet sandbox in a local setting.
//!
//! With a primary emphasis on ease of use and performance, it eliminates the
//! usage of intermediate JSON-RPC layer, allowing direct interaction with the
//! Starknet Devnet and eliminating the need for expensive serialisation and
//! deserialisation calls.
//!
//! Key Features:
//! - **Environment Handling**: Detailed setup and control mechanisms for
//!   running the Starknet-like blockchain environment.
//! - **Middleware Implementation**: Customized middleware to reduce overhead
//!   and provide optimal performance.
//!
//! For a detailed guide on getting started, check out the
//! [Starkbiter Github page](https://github.com/astraly-labs/starkbiter/).
//!
//! For specific module-level information and examples, navigate to the
//! respective module documentation below.

#![warn(missing_docs)]

pub mod environment;
pub mod errors;
pub mod middleware;
pub mod tokens;

use std::{fmt::Debug, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Sender as BroadcastSender;
use tracing::{error, info, trace, warn};

use crate::errors::StarkbiterCoreError;
