use crate::{Files, MASK};
use std::{fs, os::unix::fs::FileExt, path::PathBuf, process::exit};

pub fn main(files: &Files, seed_file: &PathBuf, force: bool) {
    if !force {
        println!("This is the only mode that will write data to your seed file.");
        println!("If you are sure you want to do this, specify the --force flag.");
        exit(2);
    }

    let writeable_seed = match fs::File::options().read(true).write(true).open(seed_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("overmask: couldn't open seed file: {error}");
            exit(1);
        }
    };
    let mut overlay_buffer = vec![0; files.block_size as usize];
    let mut mask_buffer = vec![0; files.block_size as usize];
    let mut blocks_applied = 0;

    let mut last_percent = 0.0;
    let block_limit = files.mask_size / u64::from(files.block_size);
    for block in 0..block_limit {
        #[allow(clippy::cast_precision_loss)]
        let percent = block as f64 / block_limit as f64 * 100.0;
        if percent - last_percent > 0.1 {
            last_percent = percent;
            println!("applying blocks: {percent:.1}% ({block}/{block_limit})");
        }
        let offset = block * u64::from(files.block_size);

        if let Err(error) = files.mask.read_at(&mut mask_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from mask file at offset {offset}: {error}",
                files.block_size,
            );
            if !files.ignore_errors {
                exit(1);
            }
        }
        if mask_buffer.iter().all(|&byte| byte == 0) {
            continue;
        };

        if let Err(error) = files.overlay.read_at(&mut overlay_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from overlay file at offset {offset}: {error}",
                files.block_size,
            );
            if !files.ignore_errors {
                exit(1);
            }
        }

        let mut buffer = Vec::with_capacity(files.block_size as usize);
        let mut possible_start = None;
        for (i, (mask, overlay)) in mask_buffer.iter().zip(&overlay_buffer).enumerate() {
            if mask == &MASK {
                if possible_start.is_none() {
                    possible_start = Some(i);
                }
                buffer.push(*overlay);
            } else if let Some(start) = possible_start {
                if let Err(error) = writeable_seed.write_all_at(&buffer, offset + start as u64) {
                    eprintln!(
                        "overmask: couldn't write {} bytes to seed file at offset {}: {error}",
                        buffer.len(),
                        offset + start as u64
                    );
                    if !files.ignore_errors {
                        exit(1)
                    }
                }
                buffer.clear();
                possible_start = None;
            }
        }
        if let Some(start) = possible_start {
            if let Err(error) = writeable_seed.write_all_at(&buffer, offset + start as u64) {
                eprintln!(
                    "overmask: couldn't write {} bytes to seed file at offset {}: {error}",
                    buffer.len(),
                    offset + start as u64
                );
                if !files.ignore_errors {
                    exit(1)
                }
            }
        }

        blocks_applied += 1;
    }
    println!(
        "successfully applied {blocks_applied} blocks ({} bytes) to seed",
        blocks_applied * files.block_size
    );
}
