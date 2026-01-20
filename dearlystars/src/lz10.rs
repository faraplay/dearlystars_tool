use std::io::{Error, ErrorKind, Read, Seek};

use binrw::BinRead;

use crate::util::Result;

trait ByteReader {
    fn get_byte(&mut self) -> Result<u8>;
}

impl<T: Read> ByteReader for std::io::Bytes<&mut T> {
    fn get_byte(&mut self) -> Result<u8> {
        Ok(self
            .next()
            .unwrap_or(Err(Error::from(ErrorKind::UnexpectedEof)))?)
    }
}

pub fn decompress(reader: &mut (impl Read + Seek)) -> Result<Vec<u8>> {
    let header = u32::read_le(reader)?;
    if header & 0xFF != 0x10 {
        return Err(std::io::Error::other("LZ10 header magic is wrong!").into());
    }

    let decompressed_size = header >> 8;
    let mut bytes = reader.bytes();
    let mut out_buffer = Vec::<u8>::with_capacity(decompressed_size as usize);
    loop {
        let mut flags = bytes.get_byte()?;
        for _ in 0..8 {
            if flags & 0x80 != 0 {
                let ref0 = bytes.get_byte()?;
                let ref1 = bytes.get_byte()?;
                let length = (ref0 >> 4) + 3;
                let disp = ((ref0 as u16 & 0xF) << 8) + (ref1 as u16) + 1;
                // if disp == 1 {
                //     panic!("For some reason this never occurs");
                // }
                let mut source_index = out_buffer.len() - (disp as usize);
                for _ in 0..length {
                    out_buffer.push(out_buffer[source_index]);
                    source_index += 1;
                }
            } else {
                let byte = bytes.get_byte()?;
                out_buffer.push(byte);
            }
            flags <<= 1;
            match (decompressed_size as usize).cmp(&out_buffer.len()) {
                std::cmp::Ordering::Less => {
                    return Err(std::io::Error::other("Wrote more bytes than expected!").into());
                }
                std::cmp::Ordering::Equal => {
                    return Ok(out_buffer);
                }
                std::cmp::Ordering::Greater => (),
            }
        }
    }
}

const MAX_REF_LEN: usize = 0x0F + 3;
const MAX_REF_DISP: usize = 0x0FFF + 1;

fn match_length(test_slice: &[u8], my_slice: &[u8]) -> usize {
    let test_length = test_slice.len();
    for i in 0..test_length {
        if my_slice[i] != test_slice[i] {
            return i;
        }
    }
    return test_length;
}

pub fn compress(reader: &mut (impl Read + Seek)) -> Result<Vec<u8>> {
    let mut in_buffer = Vec::<u8>::new();
    reader.read_to_end(&mut in_buffer)?;

    let decompressed_bytes: &[u8] = in_buffer.as_slice();
    let decompressed_size = decompressed_bytes.len();
    if decompressed_size > 0x00FFFFFF {
        return Err(std::io::Error::other("Decompressed file is too large!").into());
    }

    let mut out_buffer = vec![
        0x10u8,
        (decompressed_size & 0xFF) as u8,
        ((decompressed_size >> 8) & 0xFF) as u8,
        ((decompressed_size >> 16) & 0xFF) as u8,
    ];

    let mut position: usize = 0;
    let mut flags: u8 = 0;
    let mut flags_count = 0;
    let mut mini_buffer = Vec::<u8>::with_capacity(16);
    while position < decompressed_size {
        if flags_count == 8 {
            out_buffer.push(flags);
            out_buffer.append(&mut mini_buffer);
            flags = 0;
            flags_count = 0;
        }
        let test_length = std::cmp::min(decompressed_size - position, MAX_REF_LEN);
        let test_slice = &decompressed_bytes[position..position + test_length];
        let max_disp = std::cmp::min(position, MAX_REF_DISP);

        let mut best_disp = 0;
        let mut best_length = 2;
        // for disp in (1..(max_disp + 1)) {
        // For some reason disp=1 is not allowed
        for disp in (2..(max_disp + 1)).rev() {
            let this_length = match_length(test_slice, &decompressed_bytes[position - disp..]);
            if this_length > best_length {
                best_length = this_length;
                best_disp = disp;
                if best_length == test_length {
                    break;
                }
            }
        }
        flags <<= 1;
        flags_count += 1;
        if best_length > 2 {
            mini_buffer.push((((best_length - 3) << 4) | ((best_disp - 1) >> 8)) as u8);
            mini_buffer.push(((best_disp - 1) & 0xFF) as u8);
            position += best_length;
            flags |= 1;
        } else {
            mini_buffer.push(decompressed_bytes[position]);
            position += 1;
        }
    }
    flags <<= 8 - flags_count;
    out_buffer.push(flags);
    out_buffer.append(&mut mini_buffer);

    Ok(out_buffer)
}
