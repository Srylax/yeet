use std::{env::current_dir, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use lipgloss::{Color, Style};
use lipgloss_extras::table::Table;
use serde::{Deserialize, Serialize};
use url::Url;

pub fn table() -> Table {
    let style_func = move |row: i32, _col: usize| -> Style {
        match row {
            -1_i32 => Style::new().bold(true).margin_right(2),
            _ => Style::new().margin_right(2),
        }
    };
    Table::new()
        .wrap(true)
        .border_bottom(false)
        .border_left(false)
        .border_right(false)
        .border_top(false)
        .border_column(false)
        .border_row(false)
        .border_style(Style::new().foreground(Color::from("214")))
        .style_func(style_func)
}

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
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub url: Url,
    pub httpsig_key: PathBuf,
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
    /// Register a new host
    Register {
        /// Pub key of the client
        #[arg(long)]
        host_key: PathBuf,

        /// Store path of the first version
        #[arg(long)]
        store_path: String,

        /// Pet name for the host
        #[arg(long)]
        name: Option<String>,
    },
    /// Update a host e.g. push a new store_path TODO: batch update
    Update {
        /// Pub key of the client
        #[arg(long)]
        host_key: PathBuf,

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
    /// Run you hosts inside a vm
    VM {
        /// NixOs host to run and build
        #[arg(index = 1)]
        host: String,
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
    },
}
