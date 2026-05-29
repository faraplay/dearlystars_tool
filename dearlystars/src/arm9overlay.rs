use crate::util::{Error, Result};
use encoding_rs::SHIFT_JIS;
use nom::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Seek;
use std::io::Write;
use std::path::PathBuf;

use crate::csv::parse_csv;

/// Apply translations to text within files containing executible code
///
/// Arguments:
/// * `file`     - binary file being written to (class type File)
/// * `csv_data` - data from CSV containing translation, offset within file, and max byte length
/// * `row`      - counting variable passed in to determine the Row for accessing csv_data
fn apply_translation(mut file: &File, csv_data: &Vec<Vec<String>>, row: usize) -> Result<()> {
    //grab data from CSV
    let translation_str = &csv_data[row][1]; //grab TL Text
    if translation_str.is_empty() {
        // skip string injection if there is no translation
        return Ok(());
    }

    let hex_offset_str = &csv_data[row][3]; //grab Offset in File
    // try to parse the string as a hexadecimal integer
    let Ok(offset) = u64::from_str_radix(hex_offset_str.trim_start_matches("0x"), 16) else {
        return Err(Error::StringInjectionDataError(format!(
            "Could not parse {hex_offset_str} as a hexadecimal integer!"
        )));
    };

    let length_str = &csv_data[row][4]; //grab Max Bytes
    // try to parse the string as an integer
    let Ok(max_bytes) = length_str.parse::<usize>() else {
        return Err(Error::StringInjectionDataError(format!(
            "Could not parse {length_str} as an integer!"
        )));
    };

    // convert the translated string to a vector of bytes using the SHIFT_JIS encoding
    let mut tl_bytes: Vec<u8> = SHIFT_JIS.encode(&translation_str).0.into();
    // add a NUL terminator byte
    tl_bytes.push(0);

    // if translated string is too long, abort string injection and print warning message
    if tl_bytes.len() > max_bytes {
        return Err(Error::StringInjectionDataError(format!(
            "Translated string\n\
            {translation_str}\n\
            is too long to inject into position {offset:#X}!\n\
            Translated string length is {}, max length is {max_bytes}",
            tl_bytes.len() - 1
        )));
    }

    //seek to location in file and WRITE a bunch of NUL bytes to clear out the space
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&vec![0; max_bytes])?;

    //seek to location in file and WRITE the Translated Bytes
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&tl_bytes)?;

    return Ok(());
}

/// Implement the command called from main.rs to apply translations to text within
///          files containing executible code
///
/// Arguments:
/// * `in_dir`       - directory containing 'arm9.bin' and the 'overlay' folder
/// * `in_csv_dir`   - directory containing CSV translation files
pub fn arm9overlay(in_dir: &PathBuf, in_csv_dir: &PathBuf) -> Result<()> {
    //Open CSV for Parsing
    let mut csv_path = in_csv_dir.clone();
    csv_path.push("ENGLISH_IMASDS_Arm9&Overlays_Translation.xlsx - ARM9Overlay_Text.csv"); //path becomes: translated_csv/filename

    let csv_data_str = std::fs::read_to_string(&csv_path)?;
    let (_, csv_data) = parse_csv
        .parse_complete(&csv_data_str)
        .map_err(|err| Error::CsvParseError(err.to_owned().into()))?;
    //csv_data is entire CSV in memory now
    //accessed by: csv_data[row][col]

    // Sort CSV rows into different buckets by filename

    // Initialise a hashmap of the buckets
    // Each bucket is a vector of references to CSV rows
    let mut csv_rows_by_filename: HashMap<&str, Vec<&Vec<String>>> = HashMap::new();
    // Make sure to skip the heading row of the csv
    for row in csv_data.iter().skip(1) {
        // filename is in column C of the row
        let filename = row[2].as_str();
        // get a mutable ref to the bucket under this filename
        // if there is no bucket under this filename, we add a default object
        // (i.e. an empty vector) to the HashMap under this filename
        let bucket = csv_rows_by_filename.entry(filename).or_default();
        // add the row to the bucket
        bucket.push(row);
    }

    // now we perform string injection for each filename
    for (filename, bucket) in csv_rows_by_filename {
        // get path to file
        let file_path = if filename.starts_with("overlay") {
            // file is in the overlay folder if the filename starts with "overlay"
            in_dir.join("overlay").join(filename)
        } else {
            in_dir.join(filename)
        };
        eprintln!("Injecting into {}", file_path.display());
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;

        // inject every row in the bucket into the file
        for row in 0..bucket.len() {
            // try to apply translation, see whether it returns Ok or Err
            match apply_translation(&mut file, &bucket, row) {
                // if Ok, we don't need to do anything
                Ok(_) => {}
                // if it's a data error in the spreadsheet row, print a warning and carry on
                Err(Error::StringInjectionDataError(message)) => {
                    eprintln!("Error injecting row:\n{message}\nSkipping row...\n");
                }
                // if it's another error (e.g. file IO error), stop and pass on the error
                Err(e) => return Err(e),
            }
        }
    }

    return Ok(());
}
