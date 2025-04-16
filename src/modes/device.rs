use crate::{block_device::Virtual, Files};
use std::path::PathBuf;
use vblk::mount;

pub fn main(
    files: Files,
    nbd_device: &PathBuf,
    nbd_timeout: u64,
    print_operations: bool,
    trim_no_punch_holes: bool,
) {
    let mut virtual_block_device = Virtual {
        files,
        print_operations,
        trim_no_punch_holes,
    };
    unsafe {
        if let Err(error) = mount(&mut virtual_block_device, nbd_device, |device| {
            println!(
                "successfully opened virtual block device at {}",
                nbd_device.to_string_lossy()
            );

            if let Err(error) = device.set_timeout(std::time::Duration::from_secs(nbd_timeout)) {
                eprintln!(
                    "overmask: couldn't set virtual block device timeout to {nbd_timeout} seconds: {error}",
                );
            }

            if let Err(error) = ctrlc::set_handler(move || {
                if let Err(error) = device.unmount() {
                    eprintln!("overmask: couldn't unmount virtual block device: {error}");
                }
            }) {
                eprintln!("overmask: couldn't add ctrlc handler: {error}");
            }

            Ok(())
        }) {
            eprintln!("overmask: couldn't mount virtual block device: {error}");
        }
    };
}
