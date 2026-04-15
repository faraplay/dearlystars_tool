use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use ndstool::read_from_dir;
use ndstool::read_from_rom;
use ndstool::write_to_dir;
use ndstool::write_to_rom;

use crate::bbq::Bbq;
use crate::ez::extract_bin;
use crate::ez::read_idx;
use crate::ez::rebuild_bin;

mod bbq;
mod csv;
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
    /// Extract data from a .bbq file to a .yaml file.
    ExtractBbq {
        /// The .bbq file to extract.
        in_bbq_file: PathBuf,
        /// The output .yaml file.
        out_yaml_file: PathBuf,
    },
    /// Rebuild a .bbq file from a previously extracted .yaml file.
    BuildBbq {
        /// The input .yaml file.
        in_yaml_file: PathBuf,
        /// The name of the .bbq file to output.
        out_bbq_file: PathBuf,
    },
    /// Extract text from all .bbq files in a folder to csv files.
    ExtractBbqText {
        /// The directory containing the .bbq files to extract.
        in_bbq_dir: PathBuf,
        // The output directory to contain the csv files.
        out_dir: PathBuf,
    },
    /// Inject text from .csv files into .bbq files in a specified folder.
    /// Note that this overwrites the .bbq files.
    InjectBbqText {
        /// The directory containing the .csv files to inject.
        in_csv_dir: PathBuf,
        /// The directory containing the .bbq files to modify.
        bbq_dir: PathBuf,
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
            eprintln!("Extracted nds rom file.");
        }
        Commands::BuildNds {
            in_directory,
            out_nds_file,
        } => {
            let mut dir_source =
                read_from_dir(in_directory).expect("Error reading data from directory!");
            let mut nds_writer = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(out_nds_file)
                .expect("Error creating rom file!");
            write_to_rom(&mut dir_source, &mut nds_writer).expect("Error extracting data to rom!");
            eprintln!("Rebuilt nds rom file.");
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
            eprintln!("Extracted bin and idx file.");
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
            eprintln!("Rebuilt bin and idx file.");
        }
        Commands::ExtractBbq {
            in_bbq_file,
            out_yaml_file,
        } => {
            let mut bbq_reader = File::open(&in_bbq_file).expect("Error opening bbq file!");
            let mut yaml_writer = File::create(&out_yaml_file).expect("Error creating yaml file!");

            let bbq = Bbq::read_bbq(&mut bbq_reader).expect("Error reading bbq file!");
            for line in bbq.yaml_lines() {
                writeln!(yaml_writer, "{}", line).expect("Error writing to yaml file!");
            }
            yaml_writer.flush().expect("Error flushing yaml file!");
            eprintln!("Extracted bbq to yaml file.");
        }
        Commands::BuildBbq {
            in_yaml_file,
            out_bbq_file,
        } => {
            let yaml = std::fs::read_to_string(in_yaml_file).expect("Error reading yaml file!");
            let mut yaml_lines: Vec<&str> = yaml.split("\n").collect();
            yaml_lines.pop_if(|line| line.is_empty());

            let bbq = Bbq::from_yaml_lines(&yaml_lines).expect("Error reading yaml file!");
            std::fs::write(out_bbq_file, &bbq.bytes()).expect("Error writing bbq file!");
            eprintln!("Rebuilt bbq from yaml file.");
        }
        Commands::ExtractBbqText {
            in_bbq_dir,
            out_dir,
        } => {
            bbq::extract_text(in_bbq_dir, out_dir).expect("Error extracting text from bbq files!");
            eprintln!("Extracted text from bbq files.");
        }
        Commands::InjectBbqText {
            in_csv_dir,
            bbq_dir,
        } => {
            bbq::inject_text(in_csv_dir, bbq_dir).expect("Error injecting text into bbq files!");
            eprintln!("Injected text into bbq files.");
        }
    }
}
