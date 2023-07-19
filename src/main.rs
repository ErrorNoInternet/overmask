use clap::Parser;
use std::{fs, io::Error, os::unix::prelude::FileExt};
use vblk::{mount, BlockDevice};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Original and unmodified data will be read
    /// from the read-only seed file.
    #[arg(short, long)]
    seed: String,

    /// Any writes will be written to the overlay
    /// file and read from the overlay file instead
    /// of the seed file.
    #[arg(short, long)]
    overlay: String,

    /// The mask file contains a mask of what areas
    /// have been changed in the overlay file.
    #[arg(short, long)]
    mask: String,
}

fn main() {
    let arguments = Arguments::parse();

    let seed_file = match fs::File::open(arguments.seed) {
        Ok(seed_file) => seed_file,
        Err(error) => {
            println!("unable to open seed file: {error}");
            return;
        }
    };
    let overlay_file = match fs::File::options()
        .read(true)
        .write(true)
        .open(arguments.overlay)
    {
        Ok(overlay_file) => overlay_file,
        Err(error) => {
            println!("unable to open overlay file: {error}");
            return;
        }
    };
    let mask_file = match fs::File::options()
        .read(true)
        .write(true)
        .open(arguments.mask)
    {
        Ok(mask_file) => mask_file,
        Err(error) => {
            println!("unable to open mask file: {error}");
            return;
        }
    };

    struct VirtualBlockDevice {
        seed_file: fs::File,
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
            let mut masked = false;
            for byte in &mask_buffer {
                if byte == &1u8 {
                    masked = true;
                    break;
                }
            }
            if masked {
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
            512
        }

        fn blocks(&self) -> u64 {
            (self.seed_file.metadata().unwrap().len() / 512) as u64
        }
    }

    let mut virtual_block_device = VirtualBlockDevice {
        seed_file,
        overlay_file,
        mask_file,
    };

    unsafe {
        mount(&mut virtual_block_device, "/dev/nbd0", |device| {
            ctrlc::set_handler(move || {
                device.unmount().unwrap();
            })
            .unwrap();

            Ok(())
        })
        .unwrap();
    }
}
