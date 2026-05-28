use crate::Result;

pub fn blz_decompress(in_data: &[u8]) -> Result<Vec<u8>> {
    let in_length = in_data.len();
    let compressed_and_header_size = in_data[in_length - 8] as usize
        | (in_data[in_length - 7] as usize) << 8
        | (in_data[in_length - 6] as usize) << 16;
    let header_size = in_data[in_length - 5] as usize;
    let extend_size = in_data[in_length - 4] as usize
        | (in_data[in_length - 3] as usize) << 8
        | (in_data[in_length - 2] as usize) << 16
        | (in_data[in_length - 1] as usize) << 24;

    let mut compressed_data_index = compressed_and_header_size - header_size;
    let mut decompressed_data_index = compressed_and_header_size + extend_size;

    let mut all_bytes = in_data.to_vec();
    all_bytes.resize(in_length + extend_size, 0);
    let bytes = all_bytes
        .get_mut(in_length - compressed_and_header_size..)
        .ok_or(std::io::Error::other(
            "compressed_and_header_size is larger than total data size!",
        ))?;

    loop {
        compressed_data_index -= 1;
        let mut flags = bytes[compressed_data_index];
        for _ in 0..8 {
            if flags & 0x80 != 0 {
                compressed_data_index -= 2;
                let ref0 = bytes[compressed_data_index];
                let ref1 = bytes[compressed_data_index + 1];
                let mut length = ((ref1 >> 4) + 3) as usize;
                let disp = (((ref1 as u16 & 0xF) << 8) + (ref0 as u16) + 3) as usize;

                decompressed_data_index -= length;
                while length != 0 {
                    length -= 1;
                    bytes[decompressed_data_index + length] =
                        bytes[decompressed_data_index + disp + length];
                }
            } else {
                decompressed_data_index -= 1;
                compressed_data_index -= 1;
                bytes[decompressed_data_index] = bytes[compressed_data_index];
            }
            flags <<= 1;
            if decompressed_data_index == 0 {
                return Ok(all_bytes);
            }
        }
    }
}

const MAX_REF_LEN: usize = 0x0F + 3;
const MAX_REF_DISP: usize = 0x0FFF + 3;

fn backwards_match_length(test_slice: &[u8], my_slice: &[u8]) -> usize {
    let test_length = test_slice.len();
    let my_length = my_slice.len();
    for i in 0..test_length {
        if my_slice[my_length - 1 - i] != test_slice[test_length - 1 - i] {
            return i;
        }
    }
    return test_length;
}

pub fn blz_compress(
    decompressed_bytes: &[u8],
    min_uncompressed_region_size: usize,
) -> Result<Option<Vec<u8>>> {
    let decompressed_size = decompressed_bytes.len();

    if decompressed_size < min_uncompressed_region_size {
        return Err(std::io::Error::other(
            "Minimum uncompressed region size is larger than the whole file!",
        )
        .into());
    }

    let mut compress_buffer = Vec::new();

    let mut position: usize = decompressed_size;
    let mut flags: u8 = 0;
    let mut flags_count = 0;
    let mut mini_buffer = Vec::<u8>::with_capacity(16);

    // list of every new best total size, along with the amount of uncompressed bytes
    // when the new record was made
    let mut best_total_size: usize = decompressed_size;
    let mut best_total_sizes_list: Vec<(usize, usize)> = Vec::new();

    while position > min_uncompressed_region_size {
        if flags_count == 8 {
            compress_buffer.push(flags);
            compress_buffer.append(&mut mini_buffer);
            flags = 0;
            flags_count = 0;
        }
        let test_length = std::cmp::min(position - min_uncompressed_region_size, MAX_REF_LEN);
        let test_slice = &decompressed_bytes[position - test_length..position];
        let max_disp = std::cmp::min(decompressed_size - position, MAX_REF_DISP);

        let mut best_disp = 0;
        let mut best_length = 2;
        for disp in 3..(max_disp + 1) {
            // let this_length =
            //     backwards_match_length(test_slice, &decompressed_bytes[..position + disp]);
            // For some reason length>disp is not allowed
            let this_length = std::cmp::min(
                backwards_match_length(test_slice, &decompressed_bytes[..position + disp]),
                disp,
            );
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
            mini_buffer.push((((best_length - 3) << 4) | ((best_disp - 3) >> 8)) as u8);
            mini_buffer.push(((best_disp - 3) & 0xFF) as u8);
            position -= best_length;
            flags |= 1;
        } else {
            position -= 1;
            mini_buffer.push(decompressed_bytes[position]);
        }

        // calculate what the total size would be if we stopped at this point
        // (not counting the header)

        // total size = uncompressed bytes + out_buffer + 1 (for the flags byte) + mini_buffer
        let total_size = position + compress_buffer.len() + 1 + mini_buffer.len();

        // if this is the new smallest then save what everything looks like at this point
        if total_size < best_total_size {
            best_total_size = total_size;
            best_total_sizes_list.push((position, total_size));
        }
    }

    compress_buffer.push(flags << (8 - flags_count));
    compress_buffer.append(&mut mini_buffer);

    let total_size = min_uncompressed_region_size + compress_buffer.len();
    let better_position;
    let better_total_size;
    if total_size > best_total_size {
        // if compressing all the data does not give the best total size,
        // then use the *first* position on the list that has a strictly better total size.
        // For some reason we don't just use the best total size possible.
        (better_position, better_total_size) = *best_total_sizes_list
            .iter()
            .find(|(_, better_total_size)| *better_total_size < total_size)
            .unwrap();
        compress_buffer.truncate(better_total_size - better_position);
    } else {
        better_position = min_uncompressed_region_size;
        better_total_size = total_size;
    }

    if better_total_size.next_multiple_of(4) + 8 >= decompressed_size {
        // file cannot be compressed to a smaller size
        return Ok(None);
    }

    compress_buffer.reverse();
    let pad_byte_count = better_total_size.next_multiple_of(4) - better_total_size;
    compress_buffer.extend(std::iter::repeat_n(0xFF, pad_byte_count));

    let header_size = pad_byte_count + 8;
    let compressed_and_header_size = compress_buffer.len() + 8;

    let mut out_buffer = decompressed_bytes[..better_position].to_vec();
    out_buffer.append(&mut compress_buffer);
    let extend_size = decompressed_size - (out_buffer.len() + 8);

    // write blz header
    if compressed_and_header_size > 0x00FFFFFF {
        return Err(
            std::io::Error::other("compressed_and_header_size is larger than 24 bits!").into(),
        );
    }
    if extend_size > 0xFFFFFFFF {
        return Err(std::io::Error::other("extend_size is larger than 32 bits!").into());
    }
    out_buffer.push((compressed_and_header_size & 0xFF) as u8);
    out_buffer.push(((compressed_and_header_size >> 8) & 0xFF) as u8);
    out_buffer.push(((compressed_and_header_size >> 16) & 0xFF) as u8);
    out_buffer.push(header_size as u8);
    out_buffer.push((extend_size & 0xFF) as u8);
    out_buffer.push(((extend_size >> 8) & 0xFF) as u8);
    out_buffer.push(((extend_size >> 16) & 0xFF) as u8);
    out_buffer.push(((extend_size >> 24) & 0xFF) as u8);

    Ok(Some(out_buffer))
}
