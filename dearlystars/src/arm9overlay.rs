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

/// Inject a translated string into a file containing executable code
///
/// Arguments:
/// * `file`     - binary file being written to
/// * `row`      - one row of the CSV containing the translated string,
///                offset within file, and max byte length
fn inject_string(mut file: &File, row: &Vec<String>) -> Result<()> {
    //grab data from CSV
    let translation_str = &row[1]; // grab Translated_Text (column B)
    if translation_str.is_empty() {
        // skip string injection if there is no translation
        return Ok(());
    }

    let hex_offset_str = &row[3]; // grab Text_Offset in File (column D)
    // try to parse the string as a hexadecimal integer
    let Ok(offset) = u64::from_str_radix(hex_offset_str.trim_start_matches("0x"), 16) else {
        return Err(Error::StringInjectionDataError(format!(
            "Could not parse the Text_Offset field {hex_offset_str} as a hexadecimal integer!"
        )));
    };

    let length_str = &row[4]; // grab Max Bytes (column E)
    // try to parse the string as an integer
    let Ok(max_bytes) = length_str.parse::<usize>() else {
        return Err(Error::StringInjectionDataError(format!(
            "Could not parse the Max Bytes field {length_str} as an integer!"
        )));
    };

    // convert the translated string to a vector of bytes using the SHIFT_JIS encoding
    let mut tl_bytes: Vec<u8> = SHIFT_JIS.encode(&translation_str).0.into();

    // if translated string is too long, abort string injection and print warning message
    // remember that we need 1 extra byte of space for the NUL terminator byte
    if tl_bytes.len() + 1 > max_bytes {
        return Err(Error::StringInjectionDataError(format!(
            "The Translated_Text field\n\
            {translation_str}\n\
            is too long to inject into position {offset:#X}! \
            Translated string length (not including NUL terminator byte) \
            is {} bytes, available space is {max_bytes} bytes",
            tl_bytes.len()
        )));
    }

    // add a NUL terminator byte
    tl_bytes.push(0);

    //seek to location in file and WRITE a bunch of NUL bytes to clear out the space
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&vec![0; max_bytes])?;

    //seek to location in file and WRITE the Translated Bytes
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&tl_bytes)?;

    return Ok(());
}

/// Inject translated strings from a csv file into a folder of game files
/// containing executable code
///
/// Arguments:
/// * `in_csv_path`             - path of CSV file containing strings to be inserted, filenames, offsets etc
/// * `extracted_nds_dir`       - path of folder containing extracted nds rom with executable files to patch
pub fn inject_exec_strings(in_csv_path: &PathBuf, extracted_nds_dir: &PathBuf) -> Result<()> {
    // Open CSV for Parsing
    let csv_data_str = std::fs::read_to_string(&in_csv_path)?;
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
            extracted_nds_dir.join("overlay").join(filename)
        } else {
            extracted_nds_dir.join(filename)
        };
        eprintln!("Injecting into {}", file_path.display());
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;

        // inject every row in the bucket into the file
        for row in bucket {
            // try to apply translation, see whether it returns Ok or Err
            match inject_string(&mut file, row) {
                // if Ok, we don't need to do anything
                Ok(_) => {}
                // if it's a data error in the spreadsheet row, print a warning and carry on
                Err(Error::StringInjectionDataError(message)) => {
                    eprintln!("Error injecting row! Reason:\n{message}\nSkipping this row...\n");
                }
                // if it's another error (e.g. file IO error), stop and pass on the error
                Err(e) => return Err(e),
            }
        }
    }

    return Ok(());
}
