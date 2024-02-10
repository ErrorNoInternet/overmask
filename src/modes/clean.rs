use crate::Files;
use std::{os::unix::fs::FileExt, process::exit};

pub fn main(files: &Files, truncate: bool) {
    println!("deduplicating seed and overlay files...");

    let mut seed_buffer = vec![0; files.block_size as usize];
    let mut overlay_buffer = vec![0; files.block_size as usize];
    let zeros = vec![0; files.block_size as usize];
    let mut blocks_freed = 0;

    let mut last_percent = 0.0;
    let block_limit = files.seed_size / u64::from(files.block_size);
    for block in 0..block_limit {
        #[allow(clippy::cast_precision_loss)]
        let percent = block as f64 / block_limit as f64 * 100.0;
        if percent - last_percent > 0.1 {
            last_percent = percent;
            println!("comparing blocks: {:.1}% ({block}/{block_limit})", percent);
        }
        let offset = block * u64::from(files.block_size);

        if let Err(error) = files.overlay.read_at(&mut overlay_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from overlay file at offset {offset}: {error}",
                files.block_size
            );
            if !files.ignore_errors {
                exit(1);
            }
        }
        if let Err(error) = files.seed.read_at(&mut seed_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from seed file at offset {offset}: {error}",
                files.block_size
            );
            if !files.ignore_errors {
                exit(1);
            }
        }

        if seed_buffer == overlay_buffer {
            if let Err(error) = files.overlay.write_all_at(&zeros, offset) {
                eprintln!(
                    "overmask: couldn't write {} bytes to overlay file at offset {offset}: {error}",
                    files.block_size
                );
                if !files.ignore_errors {
                    exit(1);
                }
            };
            if let Err(error) = files.mask.write_all_at(&zeros, offset) {
                eprintln!(
                    "overmask: couldn't write {} bytes to mask file at offset {offset}: {error}",
                    files.block_size
                );
                if !files.ignore_errors {
                    exit(1);
                }
            };
            blocks_freed += 1;
        }
    }
    println!(
        "successfully zeroed {blocks_freed} blocks ({} bytes)",
        blocks_freed * files.block_size
    );

    if truncate {
        do_truncate(files);
    };
}

fn do_truncate(files: &Files) {
    println!("locating end of mask file...");

    let mut mask_buffer = vec![0; files.block_size as usize];
    let mut end_of_file = None;

    let mut last_percent = 0.0;
    let block_limit = files.mask_size / u64::from(files.block_size);
    for block in (0..block_limit).rev() {
        #[allow(clippy::cast_precision_loss)]
        let percent = 100.0 - block as f64 / block_limit as f64 * 100.0;
        if percent - last_percent > 0.1 {
            last_percent = percent;
            println!("checking blocks: {:.1}% ({block}/{block_limit})", percent);
        }
        let offset = block * u64::from(files.block_size);

        if let Err(error) = files.mask.read_at(&mut mask_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from mask file at offset {offset}: {error}",
                files.block_size
            );
            if !files.ignore_errors {
                exit(1);
            }
        }
        if !mask_buffer.iter().all(|&byte| byte == 0) {
            break;
        }
        end_of_file = Some(offset);
    }
    if let Some(offset) = end_of_file {
        println!("truncating overlay and mask files to {offset} bytes...");
        if let Err(error) = files.mask.set_len(offset) {
            eprintln!("overmask: couldn't truncate mask file to {offset} bytes: {error}");
            exit(1);
        };
        if let Err(error) = files.overlay.set_len(offset) {
            eprintln!("overmask: couldn't truncate overlay file to {offset} bytes: {error}");
            exit(1);
        };
        println!("successfully truncated overlay and mask files to {offset} bytes");
    } else {
        println!("no unused block found");
    }
}
