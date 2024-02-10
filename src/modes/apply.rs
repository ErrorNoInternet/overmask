use crate::{Files, MASK};
use std::{fs, os::unix::fs::FileExt, path::PathBuf, process::exit};

pub fn main(files: &Files, seed_file: &PathBuf, force: bool) {
    if !force {
        println!("This is the only mode that will write data to your seed file.\nIf you are sure you want to do this, specify the --force flag.");
        exit(2);
    }

    let writeable_seed = match fs::File::options().read(true).write(true).open(seed_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("overmask: couldn't open seed file: {error}");
            exit(1);
        }
    };
    let mut buffer = vec![0; files.block_size as usize];
    let mut overlay_buffer = vec![0; files.block_size as usize];
    let mut mask_buffer = vec![0; files.block_size as usize];
    let mut blocks_applied = 0;

    let mut last_percent = 0.0;
    let block_limit = files.seed_size / u64::from(files.block_size);
    for block in 0..block_limit {
        #[allow(clippy::cast_precision_loss)]
        let percent = block as f64 / block_limit as f64 * 100.0;
        if percent - last_percent > 0.1 {
            last_percent = percent;
            println!("applying blocks: {:.1}% ({block}/{block_limit})", percent);
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

        if let Err(error) = files.seed.read_at(&mut buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from seed file at offset {offset}: {error}",
                files.block_size,
            );
            if !files.ignore_errors {
                exit(1);
            }
        }
        if let Err(error) = files.overlay.read_at(&mut overlay_buffer, offset) {
            eprintln!(
                "overmask: couldn't read {} bytes from overlay file at offset {offset}: {error}",
                files.block_size,
            );
            if !files.ignore_errors {
                exit(1);
            }
        }

        buffer = buffer
            .iter()
            .zip(overlay_buffer.iter())
            .zip(mask_buffer.iter())
            .map(|((&seed, &overlay), &mask)| if mask == MASK { overlay } else { seed })
            .collect();
        if let Err(error) = writeable_seed.write_all_at(&buffer, offset) {
            eprintln!(
                "overmask: couldn't write {} bytes to seed file at offset {offset}: {error}",
                files.block_size
            );
            if !files.ignore_errors {
                exit(1)
            }
        }
        blocks_applied += 1;
    }
    println!(
        "successfully applied {blocks_applied} blocks ({} bytes) to seed",
        blocks_applied * files.block_size
    );
}
