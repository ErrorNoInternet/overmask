use clap::Parser;
use std::{fs, io::Write, os::unix::prelude::FileExt, process::exit};
use vblk::{mount, BlockDevice};

/// Add a writeable overlay on top of read-only files
#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Original, unmodified, read-only file
    #[arg(short, long)]
    seed_file: String,

    /// Where modified (written) data would be stored
    #[arg(short, long)]
    overlay_file: String,

    /// Where a mask of the modified data would be stored
    #[arg(short, long)]
    mask_file: String,

    /// Block size for all read/write operations
    #[arg(short, long, default_value_t = 512)]
    block_size: usize,

    /// nbd device file (requires nbd kernel module)
    #[arg(short, long, default_value = "/dev/nbd0")]
    nbd_device: String,

    /// nbd timeout in seconds
    #[arg(short = 't', long, default_value_t = 60)]
    nbd_timeout: u64,

    /// Print every IO operation (read(), write(), flush(), etc)
    #[arg(short, long, default_value_t = false)]
    print_operations: bool,

    /// Ignore IO errors from the underlying files/devices
    #[arg(short, long, default_value_t = false)]
    ignore_errors: bool,

    /// Overwrite overlay and mask with zeroes on trim()
    #[arg(short, long, default_value_t = false)]
    zero_trim: bool,

    /// Deduplicate data between the seed and overlay files/devices
    #[arg(short, long, required = false)]
    clean: bool,
}

fn main() {
    let arguments = Arguments::parse();

    let seed_file = match fs::File::open(&arguments.seed_file) {
        Ok(seed_file) => seed_file,
        Err(error) => {
            eprintln!("failed to open seed file: {error}");
            exit(1);
        }
    };
    let overlay_file = match fs::File::options()
        .read(true)
        .write(true)
        .open(&arguments.overlay_file)
    {
        Ok(overlay_file) => overlay_file,
        Err(error) => {
            eprintln!("failed to open overlay file: {error}");
            exit(1);
        }
    };
    let mask_file = match fs::File::options()
        .read(true)
        .write(true)
        .open(&arguments.mask_file)
    {
        Ok(mask_file) => mask_file,
        Err(error) => {
            eprintln!("failed to open mask file: {error}");
            exit(1);
        }
    };

    let get_size = |path: &str| -> u64 {
        if match block_utils::is_block_device(&path) {
            Ok(is_block_device) => is_block_device,
            Err(_) => false,
        } {
            match block_utils::get_device_info(path) {
                Ok(device_info) => device_info.capacity,
                Err(error) => {
                    eprintln!("failed to query block device: {error}");
                    exit(1)
                }
            }
        } else {
            match fs::File::open(path) {
                Ok(file) => match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(error) => {
                        eprintln!("failed to query file metadata: {error}");
                        exit(1)
                    }
                },
                Err(error) => {
                    eprintln!("failed to open file: {error}");
                    exit(1)
                }
            }
        }
    };
    let seed_file_size = get_size(&arguments.seed_file);
    let overlay_file_size = get_size(&arguments.overlay_file);
    let mask_file_size = get_size(&arguments.mask_file);
    println!("seed file: {seed_file_size} bytes, overlay file: {overlay_file_size} bytes, mask file: {mask_file_size} bytes");

    if arguments.clean {
        let mut seed_buffer = vec![0; arguments.block_size];
        let mut overlay_buffer = vec![0; arguments.block_size];
        let zeros = vec![0; arguments.block_size];

        let mut blocks_freed = 0;
        let mut last_percent = 0.0;
        let block_limit = seed_file_size / arguments.block_size as u64;
        for block in 0..block_limit {
            let percent = block as f64 / block_limit as f64 * 100.0;
            if percent - last_percent > 0.1 {
                last_percent = percent;
                println!("comparing blocks: {:.1}% ({block}/{block_limit})", percent);
            }

            let offset = block * arguments.block_size as u64;
            match overlay_file.read_at(&mut overlay_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to read {} bytes from overlay file at offset {offset}: {error}",
                        arguments.block_size
                    );
                    if arguments.ignore_errors {
                        continue;
                    } else {
                        exit(1)
                    }
                }
            }
            match seed_file.read_at(&mut seed_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to read {} bytes from seed file at offset {offset}: {error}",
                        arguments.block_size
                    );
                    if arguments.ignore_errors {
                        continue;
                    } else {
                        exit(1)
                    }
                }
            }
            if seed_buffer == overlay_buffer {
                match overlay_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to write {} bytes to overlay file at offset {offset}: {error}",
                            arguments.block_size
                        );
                        if arguments.ignore_errors {
                            continue;
                        } else {
                            exit(1)
                        }
                    }
                };
                match mask_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to write {} bytes to mask file at offset {offset}: {error}",
                            arguments.block_size
                        );
                        if arguments.ignore_errors {
                            continue;
                        } else {
                            exit(1)
                        }
                    }
                };
                blocks_freed += 1;
            }
        }

        println!(
            "successfully zeroed {blocks_freed} blocks ({} bytes)",
            blocks_freed * arguments.block_size
        );
        return;
    }

    struct VirtualBlockDevice {
        seed_file: fs::File,
        seed_file_size: u64,
        overlay_file: fs::File,
        mask_file: fs::File,
        arguments: Arguments,
    }

    impl BlockDevice for VirtualBlockDevice {
        fn read(&mut self, offset: u64, bytes: &mut [u8]) -> std::io::Result<()> {
            if self.arguments.print_operations {
                println!("read(offset={offset} bytes={})", bytes.len());
            }

            let mut buffer = vec![0; bytes.len()];
            let mut mask_buffer = vec![0; bytes.len()];
            match self.mask_file.read_at(&mut mask_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to read {} bytes from mask file at offset {offset}: {error}",
                        bytes.len(),
                    );
                    if !self.arguments.ignore_errors {
                        return Err(error);
                    }
                }
            }
            if mask_buffer.iter().all(|&byte| byte == 0) {
                match self.seed_file.read_at(&mut buffer, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to read {} bytes from seed file at offset {offset}: {error}",
                            bytes.len(),
                        );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                }
            } else if mask_buffer.iter().all(|&byte| byte == 255) {
                match self.overlay_file.read_at(&mut buffer, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to read {} bytes from overlay file at offset {offset}: {error}",
                            bytes.len(),
                        );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                }
            } else {
                match self.seed_file.read_at(&mut buffer, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to read {} bytes from seed file at offset {offset}: {error}",
                            bytes.len(),
                        );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                }
                let mut overlay_buffer = vec![0; bytes.len()];
                match self.overlay_file.read_at(&mut overlay_buffer, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to read {} bytes from overlay file at offset {offset}: {error}",
                            bytes.len(),
                        );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                }

                buffer = buffer
                    .iter()
                    .zip(overlay_buffer.iter())
                    .zip(mask_buffer.iter())
                    .map(|((&seed, &overlay), &mask)| if mask == 255 { overlay } else { seed })
                    .collect();
            };

            bytes.copy_from_slice(&buffer[..]);
            Ok(())
        }

        fn write(&mut self, offset: u64, bytes: &[u8]) -> std::io::Result<()> {
            if self.arguments.print_operations {
                println!("write(offset={offset} bytes={})", bytes.len());
            }

            match self.overlay_file.write_all_at(bytes, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to write {} bytes to overlay file at offset {offset}: {error}",
                        bytes.len(),
                    );
                    if !self.arguments.ignore_errors {
                        return Err(error);
                    }
                }
            }
            match self.mask_file.write_all_at(&vec![255; bytes.len()], offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to write {} bytes to mask file at offset {offset}: {error}",
                        bytes.len(),
                    );
                    if !self.arguments.ignore_errors {
                        return Err(error);
                    }
                }
            };
            Ok(())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            if self.arguments.print_operations {
                println!("flush()");
            }

            match self.overlay_file.flush() {
                Ok(_) => (),
                Err(error) => {
                    eprintln!("failed to flush overlay file: {error}");
                    if !self.arguments.ignore_errors {
                        return Err(error);
                    }
                }
            }
            match self.mask_file.flush() {
                Ok(_) => (),
                Err(error) => {
                    eprintln!("failed to flush mask file: {error}");
                    if !self.arguments.ignore_errors {
                        return Err(error);
                    }
                }
            }
            Ok(())
        }

        fn trim(&mut self, offset: u64, len: u32) -> std::io::Result<()> {
            if self.arguments.print_operations {
                println!("trim(offset={offset} len={len})");
            }

            if self.arguments.zero_trim {
                let zeros = vec![0; len as usize];
                match self.overlay_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                        "failed to write {len} zeros to overlay file at offset {offset}: {error}"
                    );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                };
                match self.mask_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!(
                            "failed to write {len} zeros to mask file at offset {offset}: {error}"
                        );
                        if !self.arguments.ignore_errors {
                            return Err(error);
                        }
                    }
                };
            }
            Ok(())
        }

        fn unmount(&mut self) {
            if self.arguments.print_operations {
                println!("unmount()")
            }
        }

        fn block_size(&self) -> u32 {
            self.arguments.block_size as u32
        }

        fn blocks(&self) -> u64 {
            self.seed_file_size / self.arguments.block_size as u64
        }
    }

    let mut virtual_block_device = VirtualBlockDevice {
        seed_file,
        seed_file_size,
        overlay_file,
        mask_file,
        arguments: arguments.clone(),
    };
    unsafe {
        match mount(&mut virtual_block_device, &arguments.nbd_device, |device| {
            println!(
                "successfully opened virtual block device at {}",
                arguments.nbd_device
            );
            match device.set_timeout(std::time::Duration::from_secs(arguments.nbd_timeout)) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "failed to set virtual block device timeout to {} seconds: {error}",
                        arguments.nbd_timeout
                    )
                }
            };

            match ctrlc::set_handler(move || match device.unmount() {
                Ok(_) => (),
                Err(error) => {
                    eprintln!("failed to unmount virtual block device: {error}")
                }
            }) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!("failed to add ctrlc handler: {error}")
                }
            };
            Ok(())
        }) {
            Ok(_) => (),
            Err(error) => {
                eprintln!("failed to mount virtual block device: {error}")
            }
        }
    }
}
