use std::{env::current_dir, path::PathBuf};

use build::CLAP_LONG_VERSION;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use shadow_rs::shadow;
use url::Url;

shadow!(build);

#[derive(Parser)]
#[clap(long_version = CLAP_LONG_VERSION)]
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

    /// Path to ed25519 key which is used for authentication
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub httpsig_key: Option<PathBuf>, // TODO: create a key selector

    /// Cachix cache name
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cachix: Option<String>,

    /// Cachix signing key
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cachix_key: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub url: Url,
    pub httpsig_key: PathBuf,
    pub cachix: Option<String>,
    pub cachix_key: Option<String>,
}

#[expect(clippy::doc_markdown, reason = "No Markdown for clap")]
#[derive(Subcommand)]
pub enum Commands {
    Agent {
        /// Seconds to wait between updates.
        /// Lower bound, may be higher between switching versions
        #[arg(short, long, default_value = "30")]
        sleep: u64,

        /// Collect facter with nixos-facter
        #[arg(long)]
        facter: bool,
    },
    /// Build and then publish some or all hosts in a flake
    Publish {
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,

        /// Hosts to build - default is all
        #[arg(long)]
        host: Vec<String>,

        /// netrc File to use when downloading from the cache. Useful when using private caches
        #[arg(long)]
        netrc: Option<PathBuf>,

        /// Which hosts should be built? Defaults to current ARCH
        #[arg(
            long,
            default_value_t = std::env::consts::ARCH == "aarch64",
            default_missing_value = (std::env::consts::ARCH == "aarch64").to_string(),
            num_args = 0..=1,
            require_equals = false)]
        darwin: bool,
    },
    /// Build some or all hosts in a flake
    Build {
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
        /// Hosts to build - default is all
        #[arg(long)]
        host: Vec<String>,
        /// Which hosts should be built? Defaults to current ARCH
        #[arg(
            long,
            default_value_t = std::env::consts::ARCH == "aarch64",
            default_missing_value = (std::env::consts::ARCH == "aarch64").to_string(),
            num_args = 0..=1,
            require_equals = false)]
        darwin: bool,
    },

    /// Query the status of all or your local hosts
    /// Requires either admin credentials or sudo
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

        /// netrc File to use when downloading from the cache. Useful when using private caches
        #[arg(long)]
        netrc: Option<PathBuf>,
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

        /// netrc File to use when downloading from the cache. Useful when using private caches
        #[arg(long)]
        netrc: Option<api::NETRC>,

        /// Pet name for the host
        #[arg(index = 1)]
        name: String,
    },
    /// Check if a key is verified
    VerifyStatus,
    /// Adds a key to the server for verification
    AddVerification {
        /// Store path of the current running system
        #[arg(long)]
        store_path: String,
        /// The public key the of the verification attempt
        #[arg(long)]
        public_key: PathBuf,
        /// Facter input file
        #[arg(long)]
        facter: Option<PathBuf>,
    },
    /// Approve a pending key verification with the corresponding code
    VerifyAttempt {
        /// Pet name for the host
        #[arg(index = 1)]
        name: String,
        /// Verification code
        #[arg(index = 2)]
        code: u32,
        /// Facter output file
        #[arg(long)]
        facter: PathBuf,
    },
    /// Add a new admin or build key to the server
    AddKey {
        /// Public key to add
        #[arg(index = 1)]
        key: PathBuf,
        /// Should the key be added as admin or as build
        #[arg(value_enum, index = 2)]
        admin: AuthLevel,
    },
    /// Remove a key from the server (can also used to remove hosts)
    RemoveKey {
        /// Public key to remove
        #[arg(index = 1)]
        key: PathBuf,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum AuthLevel {
    /// New Admin Level key [CAUTION]
    Admin,
    /// New key for build pipelines
    Build,
}
