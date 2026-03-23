use std::fs::File;
use std::io::{BufRead, Cursor, Read, Seek, SeekFrom, Write, copy};
use std::path::{Path, PathBuf};

use binrw::{BinRead, binrw};

use crate::lz10::{compress, decompress};
use crate::util::{Error, Result, mkdir, pad_to_alignment};

fn ez_error(error_message: &str) -> Error {
    Error::EzError(error_message.to_string())
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct EzTableHeader {
    magic1: [u8; 0x8],
    something1: u16,
    magic2: u16,
    entry_count: u32,
}

#[binrw]
#[brw(little)]
#[repr(C)]
pub struct EzTableEntry {
    file_offset: u32,
    size_and_compressed_flag: u32,
    name_offset: u32,
}

impl EzTableEntry {
    fn decompressed_size(&self) -> u32 {
        self.size_and_compressed_flag & 0x0FFFFFFF
    }
    fn is_compressed(&self) -> bool {
        (self.size_and_compressed_flag & 0x10000000) != 0
    }
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct EzpHeader {
    magic1: [u8; 0x8],
    something1: u16,
    magic2: u16,
    names_offset: u32,
}

pub fn read_idx(reader: &mut (impl Read + Seek)) -> Result<Vec<EzTableEntry>> {
    let header = EzTableHeader::read(reader)?;
    if header.magic1 != [0x45, 0x5A, 0x54, 0x00, 0x0E, 0x07, 0xD9, 0x07] || header.magic2 != 0x16 {
        return Err(ez_error("IDX header is wrong!"));
    }

    let mut entries = Vec::<EzTableEntry>::new();
    for _ in 0..header.entry_count {
        entries.push(EzTableEntry::read(reader)?);
    }
    Ok(entries)
}

fn indent(count: usize) -> String {
    std::iter::repeat_n(' ', 4 * count).collect()
}

fn do_not_compress(name: &str) -> bool {
    name.ends_with(".S14")
        || name.ends_with(".SSS")
        || name.ends_with(".NFTR")
        || name.ends_with(".BIN")
        || name.ends_with(".IDX")
        || (name.ends_with(".NSBCA")
            && (name.starts_with("AI_")
                || name.starts_with("ERI_")
                || name.starts_with("RYO_")
                || name.starts_with("DNC_")
                || name.starts_with("ACT_")))
}

fn padding_alignment(name: &str) -> u32 {
    if name.ends_with(".NCLR") || name.ends_with(".NSCR") || name.ends_with(".NCER") {
        0x10
    } else {
        0x20
    }
}

pub fn extract_bin(
    reader: &mut (impl Read + Seek),
    entries: &[EzTableEntry],
    out_dir: impl AsRef<Path>,
) -> Result<()> {
    let mut buf_reader = std::io::BufReader::new(reader);
    let header = EzpHeader::read(&mut buf_reader)?;
    if header.magic1 != [0x45, 0x5A, 0x50, 0x00, 0x0E, 0x07, 0xD9, 0x07] || header.magic2 != 0x16 {
        return Err(ez_error("BIN header is wrong!"));
    }
    let mut names_buffer = Cursor::new(Vec::<u8>::new());
    buf_reader.seek(std::io::SeekFrom::Start(header.names_offset as u64))?;
    std::io::copy(&mut buf_reader, &mut names_buffer)?;

    let out_dir = out_dir.as_ref();
    mkdir(out_dir)?;
    let mut path_stack = Vec::<String>::new();
    let file_list_path: PathBuf = [out_dir, Path::new(".index")].iter().collect();
    let mut file_list_writer = File::create(&file_list_path)?;
    writeln!(&mut file_list_writer, "{:04X}", header.something1)?;
    let mut index = -1;
    for entry in entries {
        index += 1;
        let mut name_buffer = Vec::<u8>::new();
        names_buffer.seek(std::io::SeekFrom::Start(entry.name_offset as u64))?;
        names_buffer.read_until(0u8, &mut name_buffer)?;
        name_buffer.pop();
        let name =
            String::from_utf8(name_buffer).or(Err(ez_error("File name is invalid UTF8!")))?;

        if let Some(dir_name) = name
            .strip_suffix("_BEGIN")
            .or_else(|| name.strip_suffix("_START"))
        {
            if entry.decompressed_size() != 0 {
                return Err(ez_error("BEGIN/START entry has nonzero size!"));
            }
            writeln!(
                &mut file_list_writer,
                "{}{}",
                indent(path_stack.len()),
                name,
            )?;
            path_stack.push(String::from(dir_name));
            let new_dir: PathBuf = std::iter::once(out_dir)
                .chain(path_stack.iter().map(|s| Path::new(s)))
                .collect();
            mkdir(&new_dir)?;
            continue;
        }

        if let Some(dir_name) = name.strip_suffix("_END") {
            if entry.decompressed_size() != 0 {
                return Err(ez_error("END entry has nonzero size!"));
            }
            if path_stack.pop().as_ref().map(|s| s.as_str()) != Some(dir_name) {
                return Err(ez_error(
                    "END dir name does not match BEGIN/START dir name!",
                ));
            }
            writeln!(
                &mut file_list_writer,
                "{}{}",
                indent(path_stack.len()),
                name,
            )?;
            continue;
        }

        let entry_path: PathBuf = std::iter::once(out_dir)
            .chain(path_stack.iter().map(|s| Path::new(s)))
            .chain(std::iter::once(Path::new(&format!("{index:04}_{name}"))))
            .collect();

        let mut entry_writer = File::create(&entry_path)?;
        buf_reader.seek(SeekFrom::Start(entry.file_offset as u64))?;
        let decompressed_size = entry.decompressed_size();
        if entry.is_compressed() {
            let out_buffer = decompress(&mut buf_reader)?;
            entry_writer.write_all(&out_buffer)?;
        } else {
            copy(
                &mut (&mut buf_reader).take(decompressed_size as u64),
                &mut entry_writer,
            )?;
        }
        writeln!(
            &mut file_list_writer,
            "{}{}",
            indent(path_stack.len()),
            name
        )?;
    }
    Ok(())
}

pub fn rebuild_bin(
    in_dir: impl AsRef<Path>,
    bin_writer: &mut (impl Write + Seek),
    idx_writer: &mut (impl Write + Seek),
) -> Result<()> {
    bin_writer.seek(SeekFrom::Start(0))?;

    let in_dir = in_dir.as_ref();
    let index_path: PathBuf = [in_dir, Path::new(".index")].iter().collect();
    let mut index_lines = std::io::BufReader::new(File::open(index_path)?).lines();
    let something1 = match index_lines.next() {
        Some(line) => u16::from_str_radix(&line?, 16)
            .or(Err(ez_error("Could not parse first line of index as hex!")))?,
        None => return Err(ez_error("index has no lines!")),
    };

    let mut entry_names = Vec::<String>::new();
    for line in index_lines {
        entry_names.push(String::from(line?.trim_start()));
    }
    let mut names_buffer = Vec::<u8>::new();

    let header = EzpHeader {
        magic1: [0x45, 0x5A, 0x50, 0x00, 0x0E, 0x07, 0xD9, 0x07],
        something1: something1,
        magic2: 0x16,
        names_offset: 0, // fill in later
    };
    binrw::BinWrite::write(&header, bin_writer)?;

    let mut path_stack = Vec::<String>::new();
    let mut entries = Vec::<EzTableEntry>::new();
    let mut index = -1;
    for name in entry_names {
        index += 1;
        let name_offset = names_buffer.len() as u32;
        names_buffer.extend(name.as_bytes());
        names_buffer.extend(std::iter::repeat_n(
            0,
            (names_buffer.len() + 1).next_multiple_of(4) - names_buffer.len(),
        ));

        if let Some(dir_name) = name
            .strip_suffix("_BEGIN")
            .or_else(|| name.strip_suffix("_START"))
        {
            entries.push(EzTableEntry {
                file_offset: bin_writer.stream_position()? as u32,
                size_and_compressed_flag: 0,
                name_offset,
            });
            path_stack.push(String::from(dir_name));
            continue;
        }

        if let Some(dir_name) = name.strip_suffix("_END") {
            entries.push(EzTableEntry {
                file_offset: bin_writer.stream_position()? as u32,
                size_and_compressed_flag: 0,
                name_offset,
            });
            if path_stack.pop().as_ref().map(|s| s.as_str()) != Some(dir_name) {
                return Err(ez_error(
                    "END dir name does not match BEGIN/START dir name!",
                ));
            }
            continue;
        }

        let entry_path: PathBuf = std::iter::once(in_dir)
            .chain(path_stack.iter().map(|s| Path::new(s)))
            .chain(std::iter::once(Path::new(&format!("{index:04}_{name}"))))
            .collect();
        eprintln!("Adding {}", entry_path.display());

        let mut entry_reader = File::open(&entry_path)?;
        let decompressed_size = entry_reader.seek(SeekFrom::End(0))?;
        if decompressed_size > 0x0FFFFFFF {
            return Err(ez_error("File is too large to add to .bin!"));
        }
        let decompressed_size = decompressed_size as u32;
        entry_reader.seek(SeekFrom::Start(0))?;
        if decompressed_size == 0 {
            entries.push(EzTableEntry {
                file_offset: bin_writer.stream_position()? as u32,
                size_and_compressed_flag: 0,
                name_offset,
            });
        } else {
            let file_offset = pad_to_alignment(bin_writer, padding_alignment(&name), 0)?;
            let is_compressed: bool;
            if do_not_compress(&name) {
                is_compressed = false;
                std::io::copy(&mut entry_reader, bin_writer)?;
            } else {
                let compressed_buffer = compress(&mut entry_reader)?;
                let compressed_size = compressed_buffer.len() as u32;
                is_compressed = compressed_size < decompressed_size;
                if is_compressed {
                    bin_writer.write_all(&compressed_buffer)?;
                } else {
                    entry_reader.seek(SeekFrom::Start(0))?;
                    std::io::copy(&mut entry_reader, bin_writer)?;
                }
            }
            entries.push(EzTableEntry {
                file_offset,
                size_and_compressed_flag: decompressed_size
                    | if is_compressed { 0x10000000 } else { 0 },
                name_offset,
            });
        }
    }

    let names_offset = bin_writer.stream_position()? as u32;
    bin_writer.write(&names_buffer)?;
    bin_writer.seek(SeekFrom::Start(0xC))?;
    binrw::BinWrite::write_le(&names_offset, bin_writer)?;

    let idx_header = EzTableHeader {
        magic1: [0x45, 0x5A, 0x54, 0x00, 0x0E, 0x07, 0xD9, 0x07],
        something1,
        magic2: 0x16,
        entry_count: entries.len() as u32,
    };
    binrw::BinWrite::write(&idx_header, idx_writer)?;
    binrw::BinWrite::write(&entries, idx_writer)?;
    binrw::BinWrite::write(
        &EzTableEntry {
            file_offset: names_offset,
            size_and_compressed_flag: 0,
            name_offset: 0,
        },
        idx_writer,
    )?;
    Ok(())
}
