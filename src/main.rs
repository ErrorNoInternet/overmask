use clap::Parser;
use std::{fs, io::Error, os::unix::prelude::FileExt, process::exit};
use vblk::{mount, BlockDevice};

const BLOCK_SIZE: usize = 512;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Original and unmodified data will be read
    /// from the read-only seed file.
    #[arg(short, long)]
    seed_file: String,

    /// Any modifications will be written to and
    /// read from the overlay file.
    #[arg(short, long)]
    overlay_file: String,

    /// The mask file contains a mask of what areas
    /// have been modified.
    #[arg(short, long)]
    mask_file: String,

    /// nbd device file (`modprobe nbd` to load module)
    #[arg(short, long, default_value = "/dev/nbd0")]
    nbd_device: String,

    /// Removes contents that are the same in the
    /// seed file and overlay file.
    #[arg(short, long, required = false)]
    clean: bool,
}

fn main() {
    let arguments = Arguments::parse();

    let seed_file = match fs::File::open(&arguments.seed_file) {
        Ok(seed_file) => seed_file,
        Err(error) => {
            println!("failed to open seed file: {error}");
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
            println!("failed to open overlay file: {error}");
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
            println!("failed to open mask file: {error}");
            exit(1);
        }
    };

    let get_size = |path: String| -> u64 {
        if match block_utils::is_block_device(&path) {
            Ok(is_block_device) => is_block_device,
            Err(_) => false,
        } {
            match block_utils::get_device_info(path) {
                Ok(device_info) => device_info.capacity,
                Err(error) => {
                    println!("failed to query block device: {error}");
                    exit(1)
                }
            }
        } else {
            match fs::File::open(path) {
                Ok(file) => match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(error) => {
                        println!("failed to query file metadata: {error}");
                        exit(1)
                    }
                },
                Err(error) => {
                    println!("failed to open file: {error}");
                    exit(1)
                }
            }
        }
    };
    let seed_file_size = get_size(arguments.seed_file);
    let overlay_file_size = get_size(arguments.overlay_file);
    let mask_file_size = get_size(arguments.mask_file);
    println!("seed file: {seed_file_size} bytes, overlay file: {overlay_file_size} bytes, mask file: {mask_file_size} bytes");

    if arguments.clean {
        let mut seed_buffer = vec![0; BLOCK_SIZE];
        let mut overlay_buffer = vec![0; BLOCK_SIZE];
        let zeros = vec![0; BLOCK_SIZE];
        let mut blocks_freed = 0;
        let mut last_zero: (bool, u64) = (false, 0);
        for block in 0..(seed_file_size / BLOCK_SIZE as u64) {
            let offset = block * BLOCK_SIZE as u64;
            match seed_file.read_at(&mut seed_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to read {BLOCK_SIZE} bytes from seed file at offset {offset}: {error}");
                    continue;
                }
            }
            match overlay_file.read_at(&mut overlay_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to read {BLOCK_SIZE} bytes from overlay file at offset {offset}: {error}");
                    continue;
                }
            }
            if !seed_buffer.iter().all(|&byte| byte == 0) && seed_buffer == overlay_buffer {
                match overlay_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        println!("failed to write {BLOCK_SIZE} bytes to overlay file at offset {offset}: {error}");
                        continue;
                    }
                };
                match mask_file.write_at(&zeros, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        println!("failed to write {BLOCK_SIZE} bytes to mask file at offset {offset}: {error}");
                        continue;
                    }
                };
                blocks_freed += 1;
            }
        }
        for block in 0..(overlay_file_size / BLOCK_SIZE as u64) {
            let offset = block * BLOCK_SIZE as u64;
            match overlay_file.read_at(&mut overlay_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to read {BLOCK_SIZE} bytes from overlay file at offset {offset}: {error}");
                    continue;
                }
            }
            if overlay_buffer.iter().all(|&byte| byte == 0) {
                if !last_zero.0 {
                    last_zero = (true, block);
                }
            } else {
                last_zero.0 = false;
            }
        }
        if last_zero.0 {
            let truncated_size = last_zero.1 * BLOCK_SIZE as u64;
            println!(
                "truncating files from {overlay_file_size} bytes to {truncated_size} bytes..."
            );
            match overlay_file.set_len(truncated_size) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to set overlay file length to {truncated_size}: {error}")
                }
            };
            match mask_file.set_len(truncated_size) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to set mask file length to {truncated_size}: {error}")
                }
            };
        }
        println!(
            "successfully zeroed {blocks_freed} blocks ({} bytes)",
            blocks_freed * BLOCK_SIZE
        );
        return;
    }

    struct VirtualBlockDevice {
        seed_file: fs::File,
        seed_file_size: u64,
        overlay_file: fs::File,
        mask_file: fs::File,
    }
    impl BlockDevice for VirtualBlockDevice {
        fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Error> {
            println!("read(offset={offset} bytes={})", bytes.len());

            let mut buffer = vec![0; bytes.len()];
            match self.seed_file.read_at(&mut buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "failed to read {} bytes from seed file at offset {offset}: {error}",
                        bytes.len(),
                    )
                }
            }
            let mut mask_buffer = vec![0; bytes.len()];
            match self.mask_file.read_at(&mut mask_buffer, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "failed to read {} bytes from mask file at offset {offset}: {error}",
                        bytes.len(),
                    )
                }
            }
            if !mask_buffer.iter().all(|&byte| byte == 0) {
                let mut overlay_buffer = vec![0; bytes.len()];
                match self.overlay_file.read_at(&mut overlay_buffer, offset) {
                    Ok(_) => (),
                    Err(error) => {
                        println!(
                            "failed to read {} bytes from overlay file at offset {offset}: {error}",
                            bytes.len(),
                        )
                    }
                }

                buffer = buffer
                    .iter()
                    .zip(overlay_buffer.iter())
                    .zip(mask_buffer.iter())
                    .map(|((&seed, &overlay), &mask)| if mask == 1 { overlay } else { seed })
                    .collect();
            };

            bytes.copy_from_slice(&buffer[..]);
            Ok(())
        }

        fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), Error> {
            println!("write(offset={offset} bytes={})", bytes.len());

            match self.overlay_file.write_all_at(bytes, offset) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "failed to write {} bytes to overlay file at offset {offset}: {error}",
                        bytes.len(),
                    )
                }
            }
            match self.mask_file.write_all_at(&vec![1; bytes.len()], offset) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "failed to write {} bytes to mask file at offset {offset}: {error}",
                        bytes.len(),
                    )
                }
            };

            Ok(())
        }

        fn flush(&mut self) -> Result<(), Error> {
            println!("flush()");
            Ok(())
        }

        fn unmount(&mut self) {
            println!("unmount()")
        }

        fn block_size(&self) -> u32 {
            BLOCK_SIZE as u32
        }

        fn blocks(&self) -> u64 {
            self.seed_file_size / BLOCK_SIZE as u64
        }
    }

    let mut virtual_block_device = VirtualBlockDevice {
        seed_file,
        seed_file_size,
        overlay_file,
        mask_file,
    };
    unsafe {
        match mount(&mut virtual_block_device, &arguments.nbd_device, |device| {
            println!("opened virtual block device at {}", arguments.nbd_device);
            match device.set_timeout(std::time::Duration::from_secs(arguments.nbd_timeout)) {
                Ok(_) => (),
                Err(error) => {
                    println!(
                        "failed to set virtual block device timeout to {} seconds: {error}",
                        arguments.nbd_timeout
                    )
                }
            };

            match ctrlc::set_handler(move || match device.unmount() {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to unmount virtual block device: {error}")
                }
            }) {
                Ok(_) => (),
                Err(error) => {
                    println!("failed to add ctrlc handler: {error}")
                }
            };
            Ok(())
        }) {
            Ok(_) => (),
            Err(error) => {
                println!("failed to mount virtual block device: {error}")
            }
        }
    }
}
