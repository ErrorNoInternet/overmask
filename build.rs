#[path = "src/arguments.rs"]
mod arguments;

use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut command = crate::arguments::Arguments::command();
    let bin_name = "overmask";
    let out_dir = "completions";

    std::fs::create_dir_all(out_dir)?;
    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh] {
        generate_to(shell, &mut command, bin_name, out_dir)?;
    }

    Ok(())
}
