use crate::{Files, MASK};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::FileExt,
    path::PathBuf,
    process::exit,
};
use vblk::BlockDevice;

pub struct Virtual {
    pub files: Files,
    pub print_operations: bool,
    pub zero_trim: bool,
}

impl BlockDevice for Virtual {
    fn read(&mut self, offset: u64, bytes: &mut [u8]) -> io::Result<()> {
        if self.print_operations {
            println!("read(offset={offset} bytes={})", bytes.len());
        }

        let mut buffer = vec![0; bytes.len()];
        let mut mask_buffer = vec![0; bytes.len()];
        if let Err(error) = self.files.mask.read_at(&mut mask_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from mask file at offset {offset}: {error}",
                bytes.len(),
            );
            if !self.files.ignore_errors {
                return Err(error);
            }
        }
        if mask_buffer.iter().all(|&byte| byte == 0) {
            if let Err(error) = self.files.seed.read_at(&mut buffer, offset) {
                eprintln!(
                    "overmask: couldn't read {} bytes from seed file at offset {offset}: {error}",
                    bytes.len(),
                );
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }
        } else if mask_buffer.iter().all(|&byte| byte == MASK) {
            if let Err(error) = self.files.overlay.read_at(&mut buffer, offset) {
                eprintln!(
                    "overmask: couldn't read {} bytes from overlay file at offset {offset}: {error}",
                    bytes.len(),
                );
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }
        } else {
            if let Err(error) = self.files.seed.read_at(&mut buffer, offset) {
                eprintln!(
                    "overmask: couldn't read {} bytes from seed file at offset {offset}: {error}",
                    bytes.len(),
                );
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }
            let mut overlay_buffer = vec![0; bytes.len()];
            if let Err(error) = self.files.overlay.read_at(&mut overlay_buffer, offset) {
                eprintln!(
                    "overmask: couldn't read {} bytes from overlay file at offset {offset}: {error}",
                    bytes.len(),
                );
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }

            buffer = buffer
                .iter()
                .zip(overlay_buffer.iter())
                .zip(mask_buffer.iter())
                .map(|((&seed, &overlay), &mask)| if mask == MASK { overlay } else { seed })
                .collect();
        };

        bytes.copy_from_slice(&buffer[..]);
        Ok(())
    }

    fn write(&mut self, offset: u64, bytes: &[u8]) -> io::Result<()> {
        if self.print_operations {
            println!("write(offset={offset} bytes={})", bytes.len());
        }

        if let Err(error) = self.files.overlay.write_all_at(bytes, offset) {
            eprintln!(
                "overmask: couldn't write {} bytes to overlay file at offset {offset}: {error}",
                bytes.len(),
            );
            if !self.files.ignore_errors {
                return Err(error);
            }
        }
        if let Err(error) = self
            .files
            .mask
            .write_all_at(&vec![MASK; bytes.len()], offset)
        {
            eprintln!(
                "overmask: couldn't write {} bytes to mask file at offset {offset}: {error}",
                bytes.len(),
            );
            if !self.files.ignore_errors {
                return Err(error);
            }
        };
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.print_operations {
            println!("flush()");
        }

        match self.files.overlay.flush() {
            Ok(()) => (),
            Err(error) => {
                eprintln!("overmask: couldn't flush overlay file: {error}");
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }
        }
        match self.files.mask.flush() {
            Ok(()) => (),
            Err(error) => {
                eprintln!("overmask: couldn't flush mask file: {error}");
                if !self.files.ignore_errors {
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn trim(&mut self, offset: u64, len: u32) -> io::Result<()> {
        if self.print_operations {
            println!("trim(offset={offset} len={len})");
        }

        if self.zero_trim {
            let zeros = vec![0; len as usize];
            match self.files.overlay.write_at(&zeros, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "overmask: couldn't write {len} zeros to overlay file at offset {offset}: {error}"
                    );
                    if !self.files.ignore_errors {
                        return Err(error);
                    }
                }
            };
            match self.files.mask.write_at(&zeros, offset) {
                Ok(_) => (),
                Err(error) => {
                    eprintln!(
                        "overmask: couldn't write {len} zeros to mask file at offset {offset}: {error}"
                    );
                    if !self.files.ignore_errors {
                        return Err(error);
                    }
                }
            };
        }
        Ok(())
    }

    fn unmount(&mut self) {
        if self.print_operations {
            println!("unmount()");
        }
    }

    fn block_size(&self) -> u32 {
        self.files.block_size
    }

    fn blocks(&self) -> u64 {
        self.files.seed_size / u64::from(self.files.block_size)
    }
}

pub fn get_size(path: &PathBuf) -> u64 {
    if block_utils::is_block_device(path).unwrap_or(false) {
        match block_utils::get_device_info(path) {
            Ok(device_info) => device_info.capacity,
            Err(error) => {
                eprintln!("overmask: couldn't query block device: {error}");
                exit(1)
            }
        }
    } else {
        match fs::File::open(path) {
            Ok(file) => match file.metadata() {
                Ok(metadata) => metadata.len(),
                Err(error) => {
                    eprintln!("overmask: couldn't query file metadata: {error}");
                    exit(1)
                }
            },
            Err(error) => {
                eprintln!("overmask: couldn't open file: {error}");
                exit(1)
            }
        }
    }
}
