mod arguments;
mod block_device;
mod modes;

use crate::arguments::{Arguments, MainSubcommand};
use crate::block_device::get_size;
use clap::Parser;
use std::{fs, process::exit};

pub struct Files {
    pub seed: fs::File,
    pub seed_size: u64,

    pub overlay: fs::File,
    pub overlay_size: u64,

    pub mask: fs::File,
    pub mask_size: u64,

    pub block_size: u32,
    pub ignore_errors: bool,
}

fn main() {
    let arguments = Arguments::parse();

    let seed = match fs::File::open(&arguments.seed_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("overmask: couldn't open seed file: {error}");
            exit(1);
        }
    };
    let overlay = match fs::File::options()
        .read(true)
        .write(true)
        .open(&arguments.overlay_file)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("overmask: couldn't open overlay file: {error}");
            exit(1);
        }
    };
    let mask = match fs::File::options()
        .read(true)
        .write(true)
        .open(&arguments.mask_file)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("overmask: couldn't open mask file: {error}");
            exit(1);
        }
    };
    let seed_size = get_size(&arguments.seed_file);
    let overlay_size = get_size(&arguments.overlay_file);
    let mask_size = get_size(&arguments.mask_file);
    println!("seed: {seed_size} bytes, overlay: {overlay_size} bytes, mask: {mask_size} bytes");

    let files = Files {
        seed,
        seed_size,
        overlay,
        overlay_size,
        mask,
        mask_size,
        block_size: arguments.block_size,
        ignore_errors: arguments.ignore_errors,
    };
    match arguments.subcommand {
        MainSubcommand::Device {
            nbd_device,
            nbd_timeout,
            print_operations,
            zero_trim,
        } => modes::device::main(files, &nbd_device, nbd_timeout, print_operations, zero_trim),
        MainSubcommand::Clean { truncate } => modes::clean::main(&files, truncate),
    };
}
