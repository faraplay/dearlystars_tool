use std::io::{Read, Seek};

use binrw::BinRead;
use encoding_rs::SHIFT_JIS;

use crate::util::{Error, Result};

use super::{BBQ_HEADER_MAGIC, BbqHeader, BbqHeaderEntry};

use super::Bbq;
use super::{BbqDataType, ByteConvert};
use super::{Command, Type2Data, Type3Data, Type5Data, Type6Data, Type7Data};

fn bbq_error(error_message: &str) -> Error {
    Error::BbqParseError(error_message.to_string())
}

impl ByteConvert for Type2Data {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for d in self.data {
            bytes.extend(d.to_le_bytes());
        }
        bytes
    }
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self> {
        if size != 28 {
            return Err(bbq_error("Size of Type 2 data is not 28!").into());
        }
        Ok(Type2Data {
            data: [
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
            ],
        })
    }
}

impl ByteConvert for Type3Data {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for d in self.data {
            bytes.extend(d.to_le_bytes());
        }
        bytes.extend((self.children.len() as u32).to_le_bytes());
        for child in &self.children {
            for d in child {
                bytes.extend(d.to_le_bytes());
            }
        }
        bytes
    }
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self> {
        let data = [
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
        ];
        let child_count = u32::read_le(reader)?;
        if size != (32 + 16 * child_count) as u64 {
            return Err(bbq_error("Size of Type 3 data is wrong!").into());
        }
        let mut children = Vec::new();
        for _ in 0..child_count {
            children.push([
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
            ])
        }
        Ok(Type3Data { data, children })
    }
}

impl ByteConvert for Type5Data {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        if let Some(x) = self.small_data {
            bytes.extend(x.to_le_bytes());
        } else {
            for line in &self.lines {
                bytes.push(line.0);
                bytes.push(line.1);
                bytes.extend(line.2.to_le_bytes());
                bytes.extend(line.3.to_le_bytes());
                bytes.extend(line.4.to_le_bytes());
                bytes.extend(line.5.to_le_bytes());
            }
        }
        bytes
    }
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self> {
        if size == 0 {
            return Err(bbq_error("Type 5 data size is zero!").into());
        }
        if size == 4 {
            let data = u32::read_le(reader)?;
            return Ok(Type5Data {
                small_data: Some(data),
                lines: Vec::new(),
            });
        }
        if size % 16 != 0 {
            return Err(bbq_error("Type 5 data size is not a multiple of 16!").into());
        }
        let child_count = size / 16;
        let mut lines = Vec::new();
        for _ in 0..child_count {
            lines.push((
                u8::read_le(reader)?,
                u8::read_le(reader)?,
                u16::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
                u32::read_le(reader)?,
            ))
        }
        Ok(Type5Data {
            small_data: None,
            lines,
        })
    }
}

impl Command {
    fn size(&self) -> u64 {
        8 + 4 * self.args.len() as u64
    }
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.code1, self.code2];
        bytes.extend(self.arg_count.to_le_bytes());
        bytes.extend(self.place.to_le_bytes());
        for arg in &self.args {
            bytes.extend(arg.to_le_bytes());
        }
        bytes
    }
    fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let code1 = u8::read_le(reader)?;
        let code2 = u8::read_le(reader)?;
        let arg_count = u16::read_le(reader)?;
        let place = u32::read_le(reader)?;
        let mut args = Vec::new();
        for _ in 0..arg_count {
            args.push(u32::read_le(reader)?)
        }
        Ok(Command {
            code1,
            code2,
            arg_count,
            place,
            args,
        })
    }
}

impl ByteConvert for Type6Data {
    fn bytes(&self) -> Vec<u8> {
        self.commands
            .iter()
            .flat_map(|command| command.bytes())
            .collect()
    }
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self> {
        let mut bytes_read = 0;
        let mut commands = Vec::new();
        while bytes_read < size {
            let command = Command::read(reader)?;
            bytes_read += command.size();
            commands.push(command);
        }
        if bytes_read != size {
            return Err(bbq_error("Type 6 data parsing read too many bytes!").into());
        }
        Ok(Type6Data { commands })
    }
}

impl ByteConvert for Type7Data {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = SHIFT_JIS.encode(&self.text).0.into();
        bytes.push(0);
        bytes
    }
    fn read(reader: &mut (impl Read + Seek), size: u64) -> Result<Self> {
        let mut buffer = vec![0; size as usize];
        reader.read_exact(&mut buffer)?;
        if buffer[(size - 1) as usize] != 0 {
            return Err(bbq_error("Type 7 data string does not end in a null byte!").into());
        }
        let mut null_index = size as usize;
        while null_index > 0 && buffer[null_index - 1] == 0 {
            null_index -= 1;
        }
        let text = SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&buffer[..null_index])
            .ok_or(bbq_error("Error decoding text in Type 7 data!"))?
            .to_string();
        Ok(Type7Data { text })
    }
}

fn offsets_and_data<T: BbqDataType>(data: &[T]) -> (Vec<u32>, Vec<u8>) {
    let mut offsets = Vec::new();
    let mut bytes = Vec::new();
    let mut current_offset: u32 = 0;
    for item in data {
        offsets.push(current_offset);
        let data_bytes = item.bytes();
        current_offset += data_bytes.len() as u32;
        bytes.extend(data_bytes);
    }
    let data_size = bytes.len();
    let pad_amount = data_size.next_multiple_of(4) - data_size;
    bytes.extend(std::iter::repeat_n(0, pad_amount));
    (offsets, bytes)
}

fn read_bbq_data<T: BbqDataType>(
    reader: &mut (impl Read + Seek),
    entry_offset: u32,
    header_entry: &BbqHeaderEntry,
) -> Result<Vec<T>> {
    let mut offsets: Vec<u32> = Vec::new();
    reader.seek(std::io::SeekFrom::Start(
        (entry_offset + header_entry.offsets_offset) as u64,
    ))?;
    for _ in 0..header_entry.data_count {
        offsets.push(u32::read_le(reader)?);
    }
    offsets.push(header_entry.data_size);

    let boundaries = offsets
        .as_slice()
        .windows(2)
        .map(|window| (window[0] as u64, window[1] as u64));
    let data_offset = (entry_offset + header_entry.data_offset) as u64;
    let mut datas: Vec<T> = Vec::new();
    for (start, end) in boundaries {
        reader.seek(std::io::SeekFrom::Start(data_offset + start))?;
        datas.push(T::read(reader, end - start)?)
    }

    Ok(datas)
}

impl Bbq {
    pub fn bytes(&self) -> Vec<u8> {
        let mut sections: Vec<(u32, Vec<u32>, Vec<u8>)> = Vec::new();

        if let Some(data) = &self.type2data {
            let (offsets, bytes) = offsets_and_data(&data);
            sections.push((2, offsets, bytes));
        }
        if let Some(data) = &self.type3data {
            let (offsets, bytes) = offsets_and_data(&data);
            sections.push((3, offsets, bytes));
        }
        if let Some(data) = &self.type5data {
            let (offsets, bytes) = offsets_and_data(&data);
            sections.push((5, offsets, bytes));
        }
        if let Some(data) = &self.type6data {
            let (offsets, bytes) = offsets_and_data(&data);
            sections.push((6, offsets, bytes));
        }
        if let Some(data) = &self.type7data {
            let (offsets, bytes) = offsets_and_data(&data);
            sections.push((7, offsets, bytes));
        }

        let section_count = sections.len() as u32;
        let mut header = BBQ_HEADER_MAGIC.to_vec();
        let mut data: Vec<u8> = Vec::new();
        header.extend(self.datetime.to_le_bytes());
        header.extend(1u32.to_le_bytes());
        header.extend(0x18u32.to_le_bytes());
        header.extend(section_count.to_le_bytes());
        for i in 0..section_count {
            let (data_type, offsets, bytes) = &sections[i as usize];
            let offsets_offset = (section_count - i) * 0x14 + (data.len() as u32);
            let data_count = offsets.len() as u32;
            let data_offset = offsets_offset + 4 * data_count;
            let data_size = bytes.len() as u32;
            header.extend(data_type.to_le_bytes());
            header.extend(offsets_offset.to_le_bytes());
            header.extend(data_count.to_le_bytes());
            header.extend(data_offset.to_le_bytes());
            header.extend(data_size.to_le_bytes());
            for offset in offsets {
                data.extend(offset.to_le_bytes());
            }
            data.extend(bytes);
        }

        header.append(&mut data);
        for d in self.footer {
            header.extend(d.to_le_bytes());
        }
        header
    }
    pub fn read_bbq(reader: &mut (impl Read + Seek)) -> Result<Bbq> {
        reader.seek(std::io::SeekFrom::Start(0))?;

        let header = BbqHeader::read(reader)?;
        if header.magic1 != BBQ_HEADER_MAGIC || header.magic2 != 1 {
            return Err(bbq_error("BBQ header is wrong!").into());
        }

        reader.seek(std::io::SeekFrom::Start(header.entries_offset as u64))?;
        let mut header_entries = Vec::<BbqHeaderEntry>::new();
        for _ in 0..header.entry_count {
            header_entries.push(BbqHeaderEntry::read(reader)?);
        }

        let (mut type2data, mut type3data, mut type5data, mut type6data, mut type7data) =
            (None, None, None, None, None);
        let mut entry_offset = header.entries_offset;
        for header_entry in &header_entries {
            match header_entry.data_type {
                2 => {
                    type2data = Some(read_bbq_data(reader, entry_offset, header_entry)?);
                }
                3 => {
                    type3data = Some(read_bbq_data(reader, entry_offset, header_entry)?);
                }
                5 => {
                    type5data = Some(read_bbq_data(reader, entry_offset, header_entry)?);
                }
                6 => {
                    type6data = Some(read_bbq_data(reader, entry_offset, header_entry)?);
                }
                7 => {
                    type7data = Some(read_bbq_data(reader, entry_offset, header_entry)?);
                }
                _ => {}
            }
            entry_offset += 0x14;
        }

        let last_entry = header_entries
            .last()
            .ok_or(bbq_error("Bbq has no sections!"))?;
        let footer_offset = (entry_offset - 0x14) + last_entry.data_offset + last_entry.data_size;
        reader.seek(std::io::SeekFrom::Start(footer_offset as u64))?;
        let footer = [
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
            u32::read_le(reader)?,
        ];

        Ok(Bbq {
            datetime: header.datetime,
            type2data,
            type3data,
            type5data,
            type6data,
            type7data,
            footer,
        })
    }
}
