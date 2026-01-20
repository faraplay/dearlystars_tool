use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use ndstool::read_from_dir;
use ndstool::read_from_rom;
use ndstool::write_to_dir;
use ndstool::write_to_rom;

use crate::ez::extract_bin;
use crate::ez::read_idx;
use crate::ez::rebuild_bin;

mod ez;
mod lz10;
mod util;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract files and data from a .nds rom.
    ExtractNds {
        /// The .nds file to extract.
        in_nds_file: PathBuf,
        /// The output directory.
        out_directory: PathBuf,
    },
    /// Rebuild a .nds rom from a previously extracted folder.
    BuildNds {
        /// The directory to build a .nds file from.
        in_directory: PathBuf,
        /// The name of the file to output.
        out_nds_file: PathBuf,
    },
    /// Extract files from a .bin and .idx file.
    ExtractBin {
        /// The .bin file to extract.
        #[arg(short = 'b')]
        in_bin_file: PathBuf,
        /// The .idx file accompanying the .bin file.
        #[arg(short = 'i')]
        in_idx_file: PathBuf,
        /// The output directory.
        out_directory: PathBuf,
    },
    /// Rebuild a .bin and .idx file from a previously extracted folder.
    BuildBin {
        /// The input directory.
        in_directory: PathBuf,
        /// The name of the .bin file to output.
        #[arg(short = 'b')]
        out_bin_file: PathBuf,
        /// The name of the accompanying .idx file to output.
        #[arg(short = 'i')]
        out_idx_file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::ExtractNds {
            in_nds_file,
            out_directory,
        } => {
            let nds_reader = File::open(&in_nds_file).expect("Error opening nds file!");
            let mut nds_source = read_from_rom(nds_reader).expect("Error processing nds file!");
            write_to_dir(&mut nds_source, out_directory).expect("Error extracting data to dir!");
            println!("Extracted nds rom file.")
        }
        Commands::BuildNds {
            in_directory,
            out_nds_file,
        } => {
            let mut dir_source =
                read_from_dir(in_directory).expect("Error reading data from directory!");
            let mut nds_writer = File::create(out_nds_file).expect("Error creating rom file!");
            write_to_rom(&mut dir_source, &mut nds_writer).expect("Error extracting data to rom!");
            println!("Rebuilt nds rom file.")
        }
        Commands::ExtractBin {
            in_bin_file,
            in_idx_file,
            out_directory,
        } => {
            let mut idx_reader = File::open(&in_idx_file).expect("Error opening idx file!");
            let entries = read_idx(&mut idx_reader).expect("Error reading idx file!");
            let mut bin_reader = File::open(&in_bin_file).expect("Error opening bin file!");
            extract_bin(&mut bin_reader, &entries, out_directory).expect("Error reading bin file!");
            println!("Extracted bin and idx file.")
        }
        Commands::BuildBin {
            in_directory,
            out_bin_file,
            out_idx_file,
        } => {
            let mut idx_writer = File::create(&out_idx_file).expect("Error creating idx file!");
            let mut bin_writer = File::create(&out_bin_file).expect("Error creating bin file!");

            rebuild_bin(in_directory, &mut bin_writer, &mut idx_writer)
                .expect("Error rebuilding bin file!");
            println!("Rebuilt bin and idx file.")
        }
    }
}
