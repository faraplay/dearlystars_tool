use std::io::Read;
use std::io::Seek;
use std::io::Write;

use aes::Aes128;
use aes::Block;
use aes::cipher::{Array, BlockCipherEncrypt, KeyInit};

use crate::Result;
use crate::header::DsHeader;
use crate::header::DsiExtraFields;

pub fn get_key_ivs(header: &DsHeader, dsi_header: &DsiExtraFields) -> (u128, u128, u128) {
    let key_x: u128 = (u64::from_le_bytes(*b"Nintendo") as u128)
        | (u32::from_le_bytes(header.gamecode) as u128) << 64
        | (u32::from_be_bytes(header.gamecode) as u128) << 96;
    let key_y: u128 = u128::from_le_bytes(dsi_header.hmac_arm9i[0..16].try_into().unwrap());
    let key = (key_x ^ key_y)
        .wrapping_add(0xFFFEFB4E295902582A680F5F1A4F3E79)
        .rotate_left(42);

    let iv1: u128 = u128::from_le_bytes(dsi_header.hmac_arm9[0..16].try_into().unwrap());
    let iv2: u128 = u128::from_le_bytes(dsi_header.hmac_arm7[0..16].try_into().unwrap());

    (key, iv1, iv2)
}

pub fn modcrypt(
    stream: &mut (impl Read + Seek + Write),
    header: &DsHeader,
    dsi_header: &DsiExtraFields,
) -> Result<()> {
    let (key, iv1, _iv2) = get_key_ivs(header, dsi_header);

    let save_position = stream.stream_position()?;
    stream.seek(std::io::SeekFrom::Start(dsi_header.modcrypt1_start as u64))?;
    let mut buffer = vec![0u8; dsi_header.modcrypt1_size as usize];
    stream.read_exact(&mut buffer)?;
    aes_ctr(&mut buffer, key, iv1);
    stream.seek(std::io::SeekFrom::Start(dsi_header.modcrypt1_start as u64))?;
    stream.write_all(&buffer)?;
    stream.seek(std::io::SeekFrom::Start(save_position))?;

    Ok(())
}

pub fn aes_ctr(data: &mut [u8], key: u128, iv: u128) {
    if data.len() % 0x10 != 0 {
        panic!("Data length in bytes is not a multiple of 16!");
    }
    let blocks_count: u128 = (data.len() as u128) / 0x10;

    let cipher = Aes128::new(&u128_to_block(key));

    let mut counter_blocks: Vec<Block> = (iv..iv + blocks_count).map(u128_to_block).collect();
    cipher.encrypt_blocks(&mut counter_blocks);
    for (data_slice, counter_block) in std::iter::zip(data.as_chunks_mut::<16>().0, &counter_blocks)
    {
        let pad = block_to_u128(counter_block);
        let plaintext = u128::from_le_bytes(*data_slice);
        let ciphertext = pad ^ plaintext;
        *data_slice = ciphertext.to_le_bytes();
    }
}

fn block_to_u128(block: &Block) -> u128 {
    u128::from_be_bytes((*block).into())
}

fn u128_to_block(int: u128) -> Block {
    Array::from(int.to_be_bytes())
}
