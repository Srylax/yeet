use std::{env::current_dir, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Yeet {
    #[command(flatten)]
    pub config: ClapConfig,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Serialize, Deserialize)]
pub struct ClapConfig {
    /// Base URL of the Yeet Server
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// Path to the admin key
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub httpsig_key: Option<PathBuf>, // TODO: create a key selector

    /// Cachix cache name
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cachix: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub url: Url,
    pub httpsig_key: PathBuf,
    pub cachix: Option<String>,
}

#[expect(clippy::doc_markdown, reason = "No Markdown for clap")]
#[derive(Subcommand)]
pub enum Commands {
    Agent {
        /// Seconds to wait between updates.
        /// Lower bound, may be higher between switching versions
        #[arg(short, long, default_value = "30")]
        sleep: u64,
    },
    /// Build some or all hosts in a flake
    Build {
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
        /// Hosts to build - default is all
        #[arg(long)]
        host: Vec<String>,
    },

    /// Query the status of all or some (TODO) hosts [requires Admin credentials]
    Status,

    /// Run you hosts inside a vm
    VM {
        /// NixOs host to run and build
        #[arg(index = 1)]
        host: String,
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
    },

    /// These are the raw subcommands to execute functions on the server
    Server(ServerArgs),
}

#[derive(Args)]
pub struct ServerArgs {
    #[command(subcommand)]
    pub command: ServerCommands,
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Build and then publish some or all hosts in a flake
    Publish {
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
        /// Hosts to build - default is all
        #[arg(long)]
        host: Vec<String>,
    },
    /// Update a host e.g. push a new `store_path` TODO: batch update
    Update {
        /// Name of the host
        #[arg(long)]
        host: String,

        /// The new store path
        #[arg(long)]
        store_path: String,

        /// The public key the agent should use to verify the update
        #[arg(long)]
        public_key: String,

        /// The substitutor the agent should use to fetch the update
        #[arg(long)]
        substitutor: String,
    },
    /// Register a new host
    Register {
        /// Store path of the first version
        #[arg(long)]
        store_path: Option<String>,

        /// The public key the agent should use to verify the update
        #[arg(long)]
        public_key: Option<String>,

        /// The substitutor the agent should use to fetch the update
        #[arg(long)]
        substitutor: Option<String>,

        /// Pet name for the host
        #[arg(index = 1)]
        name: String,
    },
    /// Check if a key is verified
    VerifyStatus,
}
