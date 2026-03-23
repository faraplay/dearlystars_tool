use std::{
    collections::HashMap,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use binrw::binrw;
use encoding_rs::SHIFT_JIS;
use nom::Parser;

use crate::{
    csv::parse_csv,
    util::{Error, Result},
};

mod byte_convert;
mod yaml_convert;

const BBQ_HEADER_MAGIC: [u8; 8] = [0x2E, 0x42, 0x42, 0x51, 0x31, 0x2E, 0x30, 0x30];
#[binrw]
#[brw(little)]
#[repr(C)]
struct BbqHeader {
    magic1: [u8; 0x8],
    datetime: u32,
    magic2: u32,
    entries_offset: u32,
    entry_count: u32,
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct BbqHeaderEntry {
    data_type: u32,
    offsets_offset: u32,
    data_count: u32,
    data_offset: u32,
    data_size: u32,
}

#[derive(Debug, PartialEq)]
pub struct Bbq {
    datetime: u32,
    type2data: Option<Vec<Type2Data>>,
    type3data: Option<Vec<Type3Data>>,
    type5data: Option<Vec<Type5Data>>,
    type6data: Option<Vec<Type6Data>>,
    type7data: Option<Vec<Type7Data>>,
    footer: [u32; 6],
}

trait ByteConvert: Sized {
    fn bytes(&self) -> Vec<u8>;
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self>;
}

trait YamlConvert: Sized {
    fn yaml_lines(&self) -> Vec<String>;
    fn from_yaml_lines(lines: &[&str], indent: usize) -> Result<Self>;
}

trait BbqDataType: ByteConvert + YamlConvert {}

#[derive(Debug, PartialEq, Clone)]
struct Type2Data {
    data: [u32; 7],
}
impl BbqDataType for Type2Data {}

#[derive(Debug, PartialEq, Clone)]
struct Type3Data {
    data: [u32; 7],
    children: Vec<[u32; 4]>,
}
impl BbqDataType for Type3Data {}

#[derive(Debug, PartialEq, Clone)]
struct Type5Data {
    small_data: Option<u32>,
    lines: Vec<(u8, u8, u16, u32, u32, u32)>,
}
impl BbqDataType for Type5Data {}

#[derive(Debug, PartialEq, Clone)]
struct Command {
    code1: u8,
    code2: u8,
    arg_count: u16,
    place: u32,
    args: Vec<u32>,
}

#[derive(Debug, PartialEq, Clone)]
struct Type6Data {
    commands: Vec<Command>,
}
impl BbqDataType for Type6Data {}

#[derive(Debug, PartialEq, Clone)]
struct Type7Data {
    text: String,
}
impl BbqDataType for Type7Data {}

pub struct ScriptLine {
    speaker_id: u8,
    audio_flag: u8,
    audio_id: u16,
    speaker_text: String,
    line_text: String,
}

impl ScriptLine {
    pub fn to_csv_string(&self) -> String {
        format!(
            "{}\t{}\t{}\t\"{}\"\t\"{}\"",
            self.speaker_id,
            self.audio_flag,
            self.audio_id,
            self.speaker_text.replace("\"", "\"\""),
            self.line_text.replace("\"", "\"\"")
        )
    }
}

impl Bbq {
    pub fn get_strings(&self) -> Option<Vec<&str>> {
        let strings: Vec<&str> = self
            .type7data
            .as_ref()?
            .iter()
            .skip(1)
            .map(|a| a.text.as_str())
            .collect();
        Some(strings)
    }
    pub fn get_script_lines(&self) -> Option<Vec<ScriptLine>> {
        let type5data = match self.type5data.as_ref()?.as_slice() {
            [a] => a,
            _ => return None,
        };
        let strings: Vec<&str> = self
            .type7data
            .as_ref()?
            .iter()
            .map(|a| a.text.as_str())
            .collect();
        let mut script_lines = Vec::new();
        for (speaker_id, audio_flag, audio_id, speaker_text_id, line_text1_id, line_text2_id) in
            &type5data.lines
        {
            script_lines.push(ScriptLine {
                speaker_id: *speaker_id,
                audio_flag: *audio_flag,
                audio_id: *audio_id,
                speaker_text: strings[*speaker_text_id as usize].to_string(),
                line_text: format!(
                    "{}\n{}",
                    strings[*line_text1_id as usize], strings[*line_text2_id as usize]
                ),
            });
        }
        Some(script_lines)
    }
    fn inject_strings(&self, strings: Vec<String>) -> Bbq {
        Bbq {
            datetime: self.datetime,
            type2data: self.type2data.clone(),
            type3data: self.type3data.clone(),
            type5data: self.type5data.clone(),
            type6data: self.type6data.clone(),
            type7data: Some(strings.into_iter().map(|text| Type7Data { text }).collect()),
            footer: self.footer,
        }
    }
    fn inject_script_lines(&self, lines: Vec<ScriptLine>) -> Bbq {
        let mut strings = vec!["".to_string()];
        let mut tuples = Vec::new();
        let line_count = lines.len() as u32;
        for line in lines {
            let speaker_text_id = get_index(&mut strings, line.speaker_text);
            let line_text1_id: usize;
            let line_text2_id: usize;
            if let Some((line_text1, line_text2)) = line.line_text.split_once("\n") {
                if line_text2.contains("\n") {
                    eprintln!(
                        "Warning! Line\n{}\ncontains more than 2 lines!",
                        &line.line_text
                    );
                }
                if sj_len(line_text1) > 40 || sj_len(line_text2) > 40 {
                    eprintln!(
                        "Warning! Line\n{}\nis over the character limit!",
                        &line.line_text
                    );
                }
                line_text1_id = get_index(&mut strings, line_text1.to_string());
                line_text2_id = get_index(&mut strings, line_text2.to_string());
            } else {
                if sj_len(&line.line_text) > 40 {
                    eprintln!(
                        "Warning! Line\n{}\nis over the character limit!",
                        &line.line_text
                    );
                }
                line_text1_id = get_index(&mut strings, line.line_text);
                line_text2_id = get_index(&mut strings, "".to_string());
            }
            tuples.push((
                line.speaker_id,
                line.audio_flag,
                line.audio_id,
                speaker_text_id as u32,
                line_text1_id as u32,
                line_text2_id as u32,
            ));
        }

        Bbq {
            datetime: self.datetime,
            type2data: Some(vec![Type2Data {
                data: [0, 65536, 0, 1, 16, line_count, 0],
            }]),
            type3data: None,
            type5data: Some(vec![Type5Data {
                small_data: None,
                lines: tuples,
            }]),
            type6data: Some(vec![Type6Data {
                commands: Vec::new(),
            }]),
            type7data: Some(strings.into_iter().map(|text| Type7Data { text }).collect()),
            footer: self.footer,
        }
    }
}

fn sj_len(text: &str) -> usize {
    SHIFT_JIS.encode(text).0.len()
}

fn get_index<T: PartialEq>(list: &mut Vec<T>, item: T) -> usize {
    match list.iter().position(|a| *a == item) {
        Some(id) => id,
        None => {
            let id = list.len();
            list.push(item);
            id
        }
    }
}

pub fn extract_text(bbq_dir: &Path, out_dir: &Path) -> Result<()> {
    let bbq_dir_basename = bbq_dir
        .file_name()
        .ok_or(std::io::Error::other("Error getting directory base name!"))?
        .to_str()
        .ok_or(std::io::Error::other("Error getting directory base name!"))?;
    eprintln!("Extracting bbqs from {bbq_dir_basename}.");
    let out_tsv_name = PathBuf::from_str(&format!("{bbq_dir_basename}.csv"))?;
    let out_mes_tsv_name = PathBuf::from_str(&format!("{bbq_dir_basename}_MES.csv"))?;
    let out_tsv_path = out_dir.join(&out_tsv_name);
    let out_mes_tsv_path = out_dir.join(&out_mes_tsv_name);
    let mut tsv_writer = std::fs::File::create(&out_tsv_path)?;
    let mut mes_tsv_writer = std::fs::File::create(&out_mes_tsv_path)?;
    writeln!(
        mes_tsv_writer,
        "Filename\tSpeaker id\tAudio flag\tAudio id\tSpeaker text\tLine text"
    )?;
    writeln!(tsv_writer, "Filename\tText",)?;
    for result in std::fs::read_dir(bbq_dir)? {
        let dir_entry = result?;
        let file_type = dir_entry.file_type()?;
        if file_type.is_dir() {
            extract_text(&dir_entry.path(), out_dir)?;
        } else if file_type.is_file() {
            let in_bbq_file = dir_entry.path();
            let filename = dir_entry
                .file_name()
                .into_string()
                .or(Err(std::io::Error::other("Error getting filename!")))?;
            if !filename.ends_with(".BBQ") {
                continue;
            }
            let mut bbq_reader = std::fs::File::open(&in_bbq_file)?;
            if let Ok(bbq) = Bbq::read_bbq(&mut bbq_reader) {
                if filename.ends_with("_MES.BBQ") {
                    let lines = bbq.get_script_lines().ok_or(std::io::Error::other(
                        "Error getting script lines from bbq!",
                    ))?;
                    for line in lines {
                        writeln!(mes_tsv_writer, "{}\t{}", filename, line.to_csv_string())?;
                    }
                } else {
                    let strings = bbq
                        .get_strings()
                        .ok_or(std::io::Error::other("Error getting strings from bbq!"))?;
                    for line in strings {
                        writeln!(
                            tsv_writer,
                            "{}\t\"{}\"",
                            filename,
                            line.replace("\"", "\"\"")
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn row_to_text(row: Vec<String>) -> Option<(String, String)> {
    let mut row_iter = row.into_iter();
    let filename = row_iter.next()?;
    let default_text = row_iter.next().unwrap_or_default();
    let replace_text = row_iter.next().unwrap_or_default();
    Some((
        filename,
        if !replace_text.is_empty() {
            replace_text
        } else {
            default_text
        },
    ))
}

fn row_to_message(row: Vec<String>) -> Option<(String, (String, String))> {
    let mut row_iter = row.into_iter();
    let filename = row_iter.next()?;
    let speaker_text = row_iter.next().unwrap_or_default();
    let default_text = row_iter.next().unwrap_or_default();
    let replace_text = row_iter.next().unwrap_or_default();
    Some((
        filename,
        (
            speaker_text,
            if !replace_text.is_empty() {
                replace_text
            } else {
                default_text
            },
        ),
    ))
}

fn get_strings_from_csv<T>(
    in_csv_path: &Path,
    row_to_t: &dyn Fn(Vec<String>) -> Option<(String, T)>,
) -> Result<HashMap<String, Vec<T>>> {
    let csv_data_str = std::fs::read_to_string(&in_csv_path)?;
    let (_, csv_data) = parse_csv
        .parse_complete(&csv_data_str)
        .map_err(|err| Error::CsvParseError(err.to_owned().into()))?;
    let mut dict = HashMap::new();
    for row in csv_data {
        if let Some((filename, text)) = row_to_t(row) {
            let entry: &mut Vec<T> = dict.entry(filename).or_default();
            entry.push(text);
        }
    }
    Ok(dict)
}

fn inject_bbq(
    bbq_path: &Path,
    file_text_dict: &mut Option<HashMap<String, Vec<String>>>,
    file_messages_dict: &mut Option<HashMap<String, Vec<(String, String)>>>,
) -> Result<()> {
    let filename = bbq_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(std::io::Error::other("Error getting filename!"))?;
    if !filename.ends_with(".BBQ") {
        return Ok(());
    }

    let mut bbq_reader = std::fs::File::open(&bbq_path)?;
    let filesize = bbq_reader.seek(std::io::SeekFrom::End(0))?;
    if filesize == 0 {
        return Ok(());
    }
    bbq_reader.seek(std::io::SeekFrom::Start(0))?;
    let bbq = Bbq::read_bbq(&mut bbq_reader)?;
    drop(bbq_reader);

    let new_bbq = if filename.ends_with("_MES.BBQ") {
        let messages_dict = match file_messages_dict {
            Some(a) => a,
            None => return Ok(()),
        };
        eprintln!("Injecting file {filename}");
        let mut script_lines = bbq.get_script_lines().ok_or(std::io::Error::other(
            "Error getting script lines from bbq!",
        ))?;
        let replace_lines = messages_dict
            .remove(filename)
            .ok_or(std::io::Error::other("Filename not found in csv!"))?;
        if replace_lines.len() != script_lines.len() {
            return Err(
                std::io::Error::other("Incorrect number of script lines for file in csv!").into(),
            );
        }
        for (script_line, (replace_speaker, replace_line)) in
            std::iter::zip(&mut script_lines, replace_lines)
        {
            script_line.speaker_text = replace_speaker;
            script_line.line_text = replace_line;
        }
        bbq.inject_script_lines(script_lines)
    } else {
        let text_dict = match file_text_dict {
            Some(a) => a,
            None => return Ok(()),
        };
        eprintln!("Injecting file {filename}");
        let bbq_texts = match &bbq.type7data {
            Some(a) => a,
            None => return Ok(()),
        };
        // eprintln!("bbq\n{bbq_texts:?}");
        if bbq_texts.len() <= 1 {
            return Ok(());
        }
        let mut replace_texts = text_dict
            .remove(filename)
            .ok_or(std::io::Error::other("Filename not found in csv!"))?;
        // eprintln!("replace\n{replace_texts:?}");
        replace_texts.insert(0, String::new());
        if replace_texts.len() != bbq_texts.len() {
            return Err(
                std::io::Error::other("Incorrect number of text entries for file in csv!").into(),
            );
        }
        bbq.inject_strings(replace_texts)
    };
    std::fs::write(bbq_path, new_bbq.bytes())?;

    Ok(())
}

pub fn inject_text(in_csv_dir: &Path, bbq_dir: &Path) -> Result<()> {
    let bbq_dir_basename = bbq_dir
        .file_name()
        .ok_or(std::io::Error::other("Error getting directory base name!"))?
        .to_str()
        .ok_or(std::io::Error::other("Error getting directory base name!"))?;
    let in_csv_name = PathBuf::from_str(&format!("{bbq_dir_basename}.csv"))?;
    let in_mes_csv_name = PathBuf::from_str(&format!("{bbq_dir_basename}_MES.csv"))?;
    let in_csv_path = in_csv_dir.join(&in_csv_name);

    let mut file_text_dict = if std::fs::exists(&in_csv_path).unwrap_or(false) {
        let dict = get_strings_from_csv(&in_csv_path, &row_to_text).ok();
        if dict.is_none() {
            eprintln!(
                "Error reading csv file {}, skipping injection for bbq files in {}...",
                in_csv_name.display(),
                bbq_dir.display()
            );
        }
        dict
    } else {
        eprintln!(
            "Csv file {} not found, skipping injection for bbq files in {}...",
            in_csv_name.display(),
            bbq_dir.display()
        );
        None
    };

    let in_mes_csv_path = in_csv_dir.join(&in_mes_csv_name);
    let mut file_messages_dict = if std::fs::exists(&in_csv_path).unwrap_or(false) {
        let dict = get_strings_from_csv(&in_mes_csv_path, &row_to_message).ok();
        if dict.is_none() {
            eprintln!(
                "Error reading csv file {}, skipping injection for message bbq files in {}...",
                in_mes_csv_name.display(),
                bbq_dir.display()
            );
        }
        dict
    } else {
        eprintln!(
            "Csv file {} not found, skipping injection for bbq files in {}...",
            in_mes_csv_name.display(),
            bbq_dir.display()
        );
        None
    };

    for result in std::fs::read_dir(bbq_dir)? {
        let dir_entry = result?;
        let file_type = dir_entry.file_type()?;
        if file_type.is_dir() {
            inject_text(in_csv_dir, &dir_entry.path())?;
        } else if file_type.is_file() {
            let bbq_path = dir_entry.path();
            inject_bbq(&bbq_path, &mut file_text_dict, &mut file_messages_dict)?;
        }
    }
    Ok(())
}
