use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Add a writeable overlay on top of read-only files
#[derive(Debug, Parser)]
#[command(version)]
pub struct Arguments {
    /// Where original read-only data would be read from
    #[arg(short, long)]
    pub seed_file: PathBuf,

    /// Where modified (written) data would be stored
    #[arg(short, long)]
    pub overlay_file: PathBuf,

    /// Where a mask of the modified data would be stored
    #[arg(short, long)]
    pub mask_file: PathBuf,

    /// Block size for all read and write operations
    #[arg(short, long, default_value_t = 512)]
    pub block_size: u32,

    /// Ignore IO errors from the underlying storage backend
    #[arg(short, long)]
    pub ignore_errors: bool,

    #[command(subcommand)]
    pub subcommand: MainSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum MainSubcommand {
    /// Create a virtual block device to capture writes
    #[command(visible_aliases = ["d", "dev"])]
    Device {
        /// nbd device file (requires nbd kernel module)
        #[arg(short, long, default_value = "/dev/nbd0")]
        nbd_device: PathBuf,

        /// nbd timeout in seconds
        #[arg(short = 't', long, default_value_t = 60)]
        nbd_timeout: u64,

        /// Print every IO operation (read(), write(), flush(), etc)
        #[arg(short, long)]
        print_operations: bool,

        /// Overwrite overlay and mask with zeroes on trim()
        #[arg(short, long)]
        zero_trim: bool,
    },

    /// Deduplicate data between the seed and overlay
    #[command(visible_aliases = ["c"])]
    Clean {
        /// Truncate the overlay and mask files to the last used byte
        #[arg(short, long)]
        truncate: bool,
    },
}
