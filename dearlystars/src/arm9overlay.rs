use crate::util::{Error, Result};
use encoding_rs::SHIFT_JIS;
use nom::Parser;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Seek;
use std::io::Write;
use std::path::PathBuf;

use crate::csv::parse_csv;

//*********************************************************************************************
// apply_translation
//
// Usage:   Apply translations to text within files containing executible code
//
// Parms:   1. file     - binary file being written to (class type File)
//          2. csv_data - data from CSV containing translation, offset within file, and max byte length
//          3. row      - counting variable passed in to determine the Row for accessing csv_data
//          4. blanks   - string of NUL (0x00) chars that a substring is made from, which is used to fill
//                        the spot where the translation is placed
//*********************************************************************************************
fn apply_translation(
    mut file: &File,
    csv_data: &Vec<Vec<String>>,
    row: usize,
    blanks: &String,
) -> Result<()> {
    //grab data from CSV
    let mut translation_str: &str = csv_data[row][1].as_str(); //grab TL Text
    if translation_str.is_empty() {
        translation_str = "---"; //if empty (imcomplete CSV? Put 3 hyphens)
    }

    let mut hex_offset_str: &str = csv_data[row][3].as_str(); //grab Offset in File
    hex_offset_str = hex_offset_str.trim_start_matches("0x");
    let offset: u64 = u64::from_str_radix(hex_offset_str, 16).unwrap();

    let length_str: &str = csv_data[row][4].as_str(); //grab Max Bytes
    let max_bytes: usize = u32::from_str_radix(length_str, 10).unwrap() as usize;

    //make a string of NUL (0x00)
    let clear_str: &str = &blanks[..max_bytes];

    //convert strings to SHIFT_JIS Vectors
    let mut tl_bytes: Vec<u8> = SHIFT_JIS.encode(&translation_str).0.into();
    let clear_bytes: Vec<u8> = SHIFT_JIS.encode(&clear_str).0.into();

    //cap translated string
    while tl_bytes.len() >= clear_bytes.len() {
        tl_bytes.pop(); //pop last character off
    }
    //loop runs until tl_bytes is at least 1 byte smaller than clear_bytes (wont run if smaller than clear_bytes)
    //the last byte will become 0x00
    tl_bytes.push(0);

    //seek to location in file and WRITE the clear bytes
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&clear_bytes)?;

    //seek to location in file and WRITE the Translated Bytes
    file.seek(std::io::SeekFrom::Start(offset))?; //move file reader to this byte
    file.write_all(&tl_bytes)?;

    return Ok(());
}

//*********************************************************************************************
// arm9overlay
//
// Usage:   Implement the command called from main.rs to apply translations to text within
//          files containing executible code
//
// Parms:   1. in_dir       - directory containing 'arm9.bin' and the 'overlay' folder
//          2. in_csv_dir   - directory containing CSV translation files
//*********************************************************************************************
pub fn arm9overlay(in_dir: &PathBuf, in_csv_dir: &PathBuf) -> Result<()> {
    // in_dir     - Directory Containing the Arm9 and Overlay Files (dearlystars_extracted)
    // in_csv_dir - Directory where CSV is (translated_csv)

    //Variables
    let mut row: usize = 1; //start index for traversing rows of CSV file

    let mut blanks: String = String::new(); //create a string of 0x00 (NUL) character to make substrings out of
    blanks.push(0u8 as char);
    blanks = blanks.repeat(300); //300 should be enough (largest is 208)

    //Open CSV for Parsing
    let mut csv_path = in_csv_dir.clone();
    csv_path.push("ENGLISH_IMASDS_Arm9&Overlays_Translation.xlsx - ARM9Overlay_Text.csv"); //path becomes: translated_csv/filename

    let csv_data_str = std::fs::read_to_string(&csv_path)?;
    let (_, csv_data): (_, Vec<Vec<String>>) = parse_csv
        .parse_complete(&csv_data_str)
        .map_err(|err| Error::CsvParseError(err.to_owned().into()))?;
    //csv_data is entire CSV in memory now
    //accessed by: csv_data[row][col]

    // --- Arm 9 ---

    //open arm9
    let mut arm_path = in_dir.clone();
    arm_path.push("arm9.bin"); //path becomes: dearlystars_extracted/arm9.bin
    let mut file = OpenOptions::new().read(true).write(true).open(&arm_path)?;

    while row < 99 {
        //loop for Arm9

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0002 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0002.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0002.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 166 {
        //loop for Overlay9 0002

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0003 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0003.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0003.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 279 {
        //loop for Overlay9 0003

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0004 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0004.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0004.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 321 {
        //loop for Overlay9 0004

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0005 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0005.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0005.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 349 {
        //loop for Overlay9 0005

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0008 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0008.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0008.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 359 {
        //loop for Overlay9 0008

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    // --- Overlay9 0009 ---
    let mut overlay_path = in_dir.clone();
    overlay_path.push("overlay/overlay9_0009.bin"); //path becomes: dearlystars_extracted/overlay/overlay9_0009.bin
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&overlay_path)?;

    while row < 394 {
        //loop for Overlay9 0009

        apply_translation(&mut file, &csv_data, row, &blanks)?;

        row = row + 1; //increment
    }

    return Ok(());
}
