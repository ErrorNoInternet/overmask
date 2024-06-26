use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Add a writeable overlay on top of read-only files
#[derive(Debug, Parser)]
#[command(version)]
pub struct Arguments {
    /// Where original read-only data should be read from
    #[arg(short, long, value_name = "FILE")]
    pub seed_file: PathBuf,

    /// Where modified (written) data should be stored
    #[arg(short, long, value_name = "FILE")]
    pub overlay_file: PathBuf,

    /// Where a mask of the modified data should be stored
    #[arg(short, long, value_name = "FILE")]
    pub mask_file: PathBuf,

    /// Block size for all read and write operations
    #[arg(short, long, value_name = "BYTES", default_value_t = 512)]
    pub block_size: u32,

    /// Ignore IO errors from the underlying files
    #[arg(short, long)]
    pub ignore_errors: bool,

    #[command(subcommand)]
    pub subcommand: MainSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum MainSubcommand {
    /// Apply the overlay on top of the seed using the mask
    #[command(visible_aliases = ["a"])]
    Apply {
        #[arg(long)]
        force: bool,
    },

    /// Deduplicate data between the seed and overlay
    #[command(visible_aliases = ["c"])]
    Clean {
        /// Truncate the overlay and mask files to the last used byte
        #[arg(short, long)]
        truncate: bool,
    },

    /// Create a virtual block device to capture writes
    #[command(visible_aliases = ["d", "dev"])]
    Device {
        /// nbd device file (requires nbd kernel module)
        #[arg(short, long, default_value = "/dev/nbd0")]
        nbd_device: PathBuf,

        /// nbd device timeout in seconds
        #[arg(short = 't', long, value_name = "SECONDS", default_value_t = 60)]
        nbd_timeout: u64,

        /// Print every IO operation (`read()`, `write()`, `flush()`, etc)
        #[arg(short, long)]
        print_operations: bool,

        /// Don't punch holes (fallocate) in overlay and mask on `trim()`
        #[arg(short = 'T', long)]
        trim_no_punch_holes: bool,
    },
}
