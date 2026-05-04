use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

use binrw::{BinReaderExt, BinWrite, BinWriterExt, binrw};
use png::BitDepth;

use crate::util::{Error, Result, mkdir};

fn gld_parse_error(error_message: String) -> Error {
    Error::GldParseError(error_message)
}

fn gld_inject_error(error_message: String) -> Error {
    Error::GldParseError(error_message)
}

pub struct Gld {
    file_stem: String,
    header: GldHeader,
    pixel_data: Vec<u8>,
    palettes: HashMap<u16, Vec<u16>>,
    footer_entries: Vec<GldFooterEntry>,
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct GldHeader {
    magic: [u8; 0x4],
    data_04: u16,
    data_06: u16,
    total_size: u32,
    data_0c: u32,
    data_10: u32,
    pixel_data_size: u32,
    palette_data_size: u32,
    footer_entry_count: u32,
}

struct GldFooterEntry {
    pixels_offset: u32,
    palette_offset: u16,
    sprite_format: u16,
    crop_width: u16,
    crop_height: u16,
    render_width_id: u16,
    render_height_id: u16,
    crop_x: u16,
    crop_y: u16,
    extra_data: GldFooterExtraData,
}

enum GldFooterExtraData {
    Type1 {
        join_x: i16,
        join_y: i16,
    },
    Type2 {
        data_14: u32,
        join_x: i16,
        join_y: i16,
    },
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct GldFooterEntry1 {
    pixels_offset: u32,
    palette_offset: u16,
    sprite_format: u16,
    crop_width: u16,
    crop_height: u16,
    render_width_id: u16,
    render_height_id: u16,
    crop_x: u16,
    crop_y: u16,
    join_x: i16,
    join_y: i16,
}

#[binrw]
#[brw(little)]
#[repr(C)]
struct GldFooterEntry2 {
    pixels_offset: u32,
    palette_offset: u16,
    sprite_format: u16,
    crop_width: u16,
    crop_height: u16,
    render_width_id: u16,
    render_height_id: u16,
    crop_x: u16,
    crop_y: u16,
    data_14: u32,
    join_x: i16,
    join_y: i16,
}

impl From<GldFooterEntry1> for GldFooterEntry {
    fn from(value: GldFooterEntry1) -> Self {
        GldFooterEntry {
            pixels_offset: value.pixels_offset,
            palette_offset: value.palette_offset,
            sprite_format: value.sprite_format,
            crop_width: value.crop_width,
            crop_height: value.crop_height,
            render_width_id: value.render_width_id,
            render_height_id: value.render_height_id,
            crop_x: value.crop_x,
            crop_y: value.crop_y,
            extra_data: GldFooterExtraData::Type1 {
                join_x: value.join_x,
                join_y: value.join_y,
            },
        }
    }
}

impl From<GldFooterEntry2> for GldFooterEntry {
    fn from(value: GldFooterEntry2) -> Self {
        GldFooterEntry {
            pixels_offset: value.pixels_offset,
            palette_offset: value.palette_offset,
            sprite_format: value.sprite_format,
            crop_width: value.crop_width,
            crop_height: value.crop_height,
            render_width_id: value.render_width_id,
            render_height_id: value.render_height_id,
            crop_x: value.crop_x,
            crop_y: value.crop_y,
            extra_data: GldFooterExtraData::Type2 {
                data_14: value.data_14,
                join_x: value.join_x,
                join_y: value.join_y,
            },
        }
    }
}

impl BinWrite for GldFooterEntry {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<()> {
        match self.extra_data {
            GldFooterExtraData::Type1 { join_x, join_y } => GldFooterEntry1 {
                pixels_offset: self.pixels_offset,
                palette_offset: self.palette_offset,
                sprite_format: self.sprite_format,
                crop_width: self.crop_width,
                crop_height: self.crop_height,
                render_width_id: self.render_width_id,
                render_height_id: self.render_height_id,
                crop_x: self.crop_x,
                crop_y: self.crop_y,
                join_x,
                join_y,
            }
            .write_options(writer, endian, args),
            GldFooterExtraData::Type2 {
                data_14,
                join_x,
                join_y,
            } => GldFooterEntry2 {
                pixels_offset: self.pixels_offset,
                palette_offset: self.palette_offset,
                sprite_format: self.sprite_format,
                crop_width: self.crop_width,
                crop_height: self.crop_height,
                render_width_id: self.render_width_id,
                render_height_id: self.render_height_id,
                crop_x: self.crop_x,
                crop_y: self.crop_y,
                data_14,
                join_x,
                join_y,
            }
            .write_options(writer, endian, args),
        }
    }
}

fn u16_to_color(x: u16) -> [u8; 3] {
    [
        ((x & 0x1F) << 3 | (x & 0x1F) >> 2) as u8,
        (((x >> 5) & 0x1F) << 3 | ((x >> 5) & 0x1F) >> 2) as u8,
        (((x >> 10) & 0x1F) << 3 | ((x >> 10) & 0x1F) >> 2) as u8,
    ]
}

fn u16_to_transparency(x: u16) -> bool {
    ((x >> 15) & 0x1) != 0
}

pub fn extract_glds(in_path: &Path, out_path: &Path) -> Result<()> {
    mkdir(out_path)?;
    let dir = std::fs::read_dir(in_path)?;
    for item in dir {
        let dir_entry = item?;

        let entry_path = dir_entry.path();
        if let Some(extension) = entry_path.extension()
            && extension.to_ascii_lowercase() == "gld"
        {
        } else {
            continue;
        }
        let mut gld_reader = File::open(&entry_path)?;
        let file_stem = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Could not get filename!");
        eprintln!("{file_stem}");
        match Gld::read(&mut gld_reader, file_stem) {
            Ok(gld) => {
                gld.extract_all_pngs(out_path)?;
            }
            Err(Error::GldParseError(e)) => eprintln!("{e} Skipping file..."),
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn inject_glds(png_dir: &Path, gld_dir: &Path, preview_dir: Option<&Path>) -> Result<()> {
    if let Some(preview_dir) = preview_dir {
        mkdir(preview_dir)?;
    }

    let indices_map = get_map_of_indices_to_replace(png_dir)?;

    for (gld_stem, indices) in &indices_map {
        let gld_path = gld_dir.join(format!("{gld_stem}.GLD"));
        let mut gld_reader = match File::open(&gld_path) {
            Ok(reader) => reader,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("Warning: File {} not found!", gld_path.display());
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        };
        let mut gld = Gld::read(&mut gld_reader, gld_stem)?;
        drop(gld_reader);

        eprintln!("Injecting pictures into {gld_stem}");
        for index in indices {
            if *index >= gld.footer_entries.len() {
                eprintln!("Warning: Index {index} is out of bounds!");
                continue;
            }
            gld.inject_png(*index, &png_dir.join(format!("{gld_stem}_{index}.png")))?;
        }

        if let Some(preview_dir) = preview_dir {
            for index in indices {
                if *index >= gld.footer_entries.len() {
                    continue;
                }
                gld.extract_png(*index, &preview_dir.join(format!("{gld_stem}_{index}.png")))?;
            }
        }

        let mut gld_writer = File::create(&gld_path)?;
        gld.write(&mut gld_writer)?;
    }

    Ok(())
}

fn get_map_of_indices_to_replace(png_path: &Path) -> Result<BTreeMap<String, Vec<usize>>> {
    let dir = std::fs::read_dir(png_path)?;
    let mut indices_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for item in dir {
        let dir_entry = item?;
        if !dir_entry.file_type()?.is_file() {
            continue;
        }
        let entry_path = dir_entry.path();
        if let Some(extension) = entry_path.extension()
            && extension.to_ascii_lowercase() == "png"
        {
        } else {
            continue;
        }
        let file_stem = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Could not get filename!");
        let Some((gld_stem, index_str)) = file_stem.rsplit_once("_") else {
            continue;
        };
        let Ok(index) = str::parse::<usize>(index_str) else {
            continue;
        };
        match indices_map.get_mut(gld_stem) {
            Some(value) => {
                value.push(index);
            }
            None => {
                indices_map.insert(gld_stem.to_string(), vec![index]);
            }
        }
    }
    Ok(indices_map)
}

impl Gld {
    pub fn read(reader: &mut (impl Read + Seek), in_file_stem: &str) -> Result<Gld> {
        let filesize = reader.seek(std::io::SeekFrom::End(0))?;
        reader.seek(std::io::SeekFrom::Start(0))?;
        if filesize == 0 {
            return Err(gld_parse_error(format!("{in_file_stem} has no data!")));
        }
        let header: GldHeader = reader.read_le()?;

        if !(header.data_04 == 2) {
            return Err(gld_parse_error(format!(
                "{in_file_stem} bad header! Has {} {}",
                header.data_04, header.data_06
            )));
        }
        if header.data_06 == 1 {
            eprintln!("{in_file_stem} has data_06 {}", header.data_06);
        }

        let mut pixel_data = vec![0u8; header.pixel_data_size as usize];
        reader.read_exact(&mut pixel_data)?;

        let mut palette_data = vec![0u8; header.palette_data_size as usize];
        reader.read_exact(&mut palette_data)?;

        let mut footer: Vec<GldFooterEntry> = Vec::new();
        match header.data_06 {
            1 => {
                for _ in 0..header.footer_entry_count {
                    footer.push(reader.read_le::<GldFooterEntry1>()?.into());
                }
            }
            2 => {
                for _ in 0..header.footer_entry_count {
                    footer.push(reader.read_le::<GldFooterEntry2>()?.into());
                }
            }
            _ => todo!(),
        }

        let palettes = footer
            .iter()
            .filter(|entry| entry.sprite_format & 0x8000 == 0)
            .map(|entry| entry.palette_offset as usize)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .chain(std::iter::once(header.palette_data_size as usize))
            .collect::<Vec<_>>()
            .windows(2)
            .map(|window| {
                (
                    window[0] as u16,
                    palette_data[window[0]..window[1]]
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|chunk| u16::from_le_bytes(*chunk))
                        .collect(),
                )
            })
            .collect();

        return Ok(Gld {
            file_stem: in_file_stem.to_string(),
            header,
            pixel_data,
            palettes,
            footer_entries: footer,
        });
    }

    pub fn write(&self, writer: &mut (impl Write + Seek)) -> Result<()> {
        writer.seek(std::io::SeekFrom::Start(0))?;
        writer.write_le(&self.header)?;
        writer.write_le(&self.pixel_data)?;

        let mut palettes = self.palettes.iter().collect::<Vec<_>>();
        palettes.sort_by_key(|(key, _value)| **key);
        for (_key, palette) in palettes {
            writer.write_le(palette)?;
        }

        writer.write_le(&self.footer_entries)?;
        Ok(())
    }

    pub fn extract_all_pngs(&self, out_dir: &Path) -> Result<()> {
        for index in 0..self.footer_entries.len() {
            self.extract_png(
                index,
                &out_dir.join(format!("{}_{index}.png", self.file_stem)),
            )?;
        }
        Ok(())
    }

    pub fn extract_png(&self, index: usize, out_png_path: &Path) -> Result<()> {
        let entry = &self.footer_entries[index];
        let var_name = entry.sprite_format & 0x8000 != 0;
        let deleted = var_name;
        debug_assert!(entry.sprite_format & 0x7F00 == 0);
        if deleted {
            return Ok(());
        }

        let bit_depth: BitDepth = match entry.sprite_format {
            1 | 4 | 6 => BitDepth::Eight,
            2 => BitDepth::Two,
            3 => BitDepth::Four,
            _ => todo!(),
        };

        let palette = &self.palettes[&entry.palette_offset];
        let (mut colors, mut transparencies) = unzip_palette(palette);
        let png_palette: Vec<u8>;
        let png_transparencies: Vec<u8>;
        match entry.sprite_format {
            1 => {
                colors.resize(32, [0, 0, 0]);
                transparencies.resize(32, false);
                png_palette = std::iter::repeat_n(&colors, 8)
                    .flatten()
                    .flatten()
                    .copied()
                    .collect();
                png_transparencies = (0..8)
                    .flat_map(|b| {
                        let alpha = b << 5 | b << 2 | b >> 1;
                        transparencies
                            .iter()
                            .map(move |is_opaque| if *is_opaque { alpha } else { 0 })
                    })
                    .collect();
            }
            2 | 3 | 4 => {
                png_palette = colors.iter().flatten().copied().collect();
                png_transparencies = transparencies
                    .iter()
                    .map(|is_opaque| if *is_opaque { 0xFF } else { 0 })
                    .collect();
            }
            6 => {
                colors.resize(8, [0, 0, 0]);
                transparencies.resize(8, false);
                png_palette = std::iter::repeat_n(&colors, 32)
                    .flatten()
                    .flatten()
                    .copied()
                    .collect();
                png_transparencies = (0..32)
                    .flat_map(|b| {
                        let alpha = b << 3 | b >> 2;
                        transparencies
                            .iter()
                            .map(move |is_opaque| if *is_opaque { alpha } else { 0 })
                    })
                    .collect();
            }
            _ => todo!(),
        };

        let max_y = entry.crop_y + entry.crop_height;
        let render_width = self.render_width(entry);
        let bit_depth_int = match bit_depth {
            BitDepth::One => 1,
            BitDepth::Two => 2,
            BitDepth::Four => 4,
            BitDepth::Eight => 8,
            BitDepth::Sixteen => 16,
        };
        let render_width_bytes = render_width as u32 * bit_depth_int / 8;
        let render_byte_count: usize = render_width_bytes as usize * max_y as usize;
        let entry_pixel_data = &self.pixel_data
            [entry.pixels_offset as usize..entry.pixels_offset as usize + render_byte_count];
        let pixels = match bit_depth {
            BitDepth::One => todo!(),
            BitDepth::Two => unpack_2bits_le(entry_pixel_data),
            BitDepth::Four => unpack_4bits_le(entry_pixel_data),
            BitDepth::Eight => entry_pixel_data.to_vec(),
            BitDepth::Sixteen => todo!(),
        };

        let mut image_data: Vec<u8> = Vec::new();
        for y in entry.crop_y..entry.crop_y + entry.crop_height {
            let start = y as usize * render_width as usize + entry.crop_x as usize;
            let end = start + entry.crop_width as usize;
            let row = &pixels[start..end];
            match bit_depth {
                BitDepth::One => todo!(),
                BitDepth::Two => image_data.extend(pack_2bits_be(row)),
                BitDepth::Four => image_data.extend(pack_4bits_be(row)),
                BitDepth::Eight => image_data.extend(row),
                BitDepth::Sixteen => todo!(),
            }
        }

        {
            // output a png
            let mut encoder = png::Encoder::new(
                File::create(out_png_path)?,
                entry.crop_width as u32,
                entry.crop_height as u32,
            );
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(bit_depth);
            encoder.set_palette(png_palette);
            encoder.set_trns(png_transparencies);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&image_data)?;
        }

        Ok(())
    }

    pub fn inject_png(&mut self, index: usize, in_png_path: &Path) -> Result<()> {
        let entry = &self.footer_entries[index];
        let max_palette_size = match entry.sprite_format {
            1 => 32,
            2 => 4,
            3 => 16,
            4 => 256,
            6 => 8,
            _ => panic!("Bad sprite format!"),
        };
        let palette_big = &self.palettes[&entry.palette_offset];
        let palette = &palette_big[..usize::min(palette_big.len(), max_palette_size)];

        let in_png = std::io::BufReader::new(File::open(in_png_path)?);
        let mut decoder = png::Decoder::new(in_png);
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let info = reader.info();

        let has_transparency = match info.color_type {
            png::ColorType::GrayscaleAlpha | png::ColorType::Rgba => true,
            png::ColorType::Grayscale | png::ColorType::Rgb => false,
            png::ColorType::Indexed => info.trns.is_some(),
        };

        let png_size = info.size();
        if png_size != (entry.crop_width as u32, entry.crop_height as u32) {
            return Err(gld_inject_error(format!(
                "Image to inject has incorrect dimensions! Expected {}x{}, got {}x{}",
                png_size.0, png_size.1, entry.crop_width, entry.crop_height
            )));
        }
        if info.is_animated() {
            return Err(gld_inject_error("Image is animated!".to_string()));
        }

        let buffer_size = reader
            .output_buffer_size()
            .expect("PNG is too large to load into memory");
        assert_eq!(
            buffer_size,
            (png_size.0 * png_size.1 * (if has_transparency { 4 } else { 3 })) as usize,
            "Unexpected number of bytes per pixel in image!"
        );

        let mut buffer = vec![0u8; buffer_size];
        reader.next_frame(&mut buffer)?;

        let pixel_color_transparencies = if has_transparency {
            buffer
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| {
                    (
                        get_color_index(chunk[0..3].try_into().unwrap(), palette) as u8,
                        chunk[3],
                    )
                })
                .collect::<Vec<_>>()
        } else {
            buffer
                .as_chunks::<3>()
                .0
                .iter()
                .map(|chunk| (get_color_index(&chunk, palette) as u8, 0xFF))
                .collect::<Vec<_>>()
        };

        let pixels: Vec<u8> = match entry.sprite_format {
            1 => pixel_color_transparencies
                .iter()
                .map(|(color, alpha)| (color & 0x1F) | (alpha & 0xE0))
                .collect(),
            2 | 3 | 4 => pixel_color_transparencies
                .iter()
                .map(|(color, _alpha)| *color)
                .collect(),
            6 => pixel_color_transparencies
                .iter()
                .map(|(color, alpha)| (color & 0x07) | (alpha & 0xF8))
                .collect(),
            _ => panic!("Bad sprite format!"),
        };

        let dest_image_data_start = entry.pixels_offset as usize;
        let image_width = entry.crop_width as usize;
        let render_width = self.render_width(entry) as usize;
        let pixels_per_byte = match entry.sprite_format {
            1 | 4 | 6 => 1,
            2 => 4,
            3 => 2,
            _ => panic!("Bad sprite format!"),
        };
        let dest_row_size_bytes =
            (image_width as usize).next_multiple_of(pixels_per_byte) / pixels_per_byte;
        for y in 0..entry.crop_height as usize {
            let src_start = y * image_width;
            let src_end = src_start + image_width;
            let dest_start = dest_image_data_start
                + (y + entry.crop_y as usize) * render_width
                + entry.crop_x as usize;
            let dest_end = dest_start + dest_row_size_bytes;
            let dest_last_byte = self.pixel_data[dest_end - 1];
            let dest = &mut self.pixel_data[dest_start..dest_end];
            match pixels_per_byte {
                1 => dest.copy_from_slice(&pixels[src_start..src_end]),
                2 => dest
                    .copy_from_slice(&pack_4bits_le(&pixels[src_start..src_end], dest_last_byte)),
                4 => dest
                    .copy_from_slice(&pack_2bits_le(&pixels[src_start..src_end], dest_last_byte)),
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    fn render_width(&self, entry: &GldFooterEntry) -> u32 {
        let max_x = entry.crop_x + entry.crop_width;
        match self.header.data_06 {
            1 => max_x.next_power_of_two() as u32,
            2 => 8 << entry.render_width_id,
            _ => todo!(),
        }
    }
}

fn unpack_2bits_le(data: &[u8]) -> Vec<u8> {
    data.iter()
        .flat_map(|x| [x & 0x3, (x >> 2) & 0x3, (x >> 4) & 0x3, (x >> 6) & 0x3])
        .collect()
}

fn unpack_4bits_le(data: &[u8]) -> Vec<u8> {
    data.iter()
        .flat_map(|x| [x & 0xF, (x >> 4) & 0xF])
        .collect()
}

fn pack_2bits_le(data: &[u8], last_byte: u8) -> Vec<u8> {
    let (chunks, remainder) = data.as_chunks::<4>();
    chunks
        .iter()
        .map(|a| (a[0] & 0x3) | (a[1] & 0x3) << 2 | (a[2] & 0x3) << 4 | (a[3] & 0x3) << 6)
        .chain(if remainder.is_empty() {
            None
        } else {
            Some(
                (remainder.get(0).copied().unwrap_or(last_byte) & 0x3)
                    | (remainder.get(1).copied().unwrap_or(last_byte >> 2) & 0x3) << 2
                    | (remainder.get(2).copied().unwrap_or(last_byte >> 4) & 0x3) << 4
                    | (remainder.get(3).copied().unwrap_or(last_byte >> 6) & 0x3) << 6,
            )
        })
        .collect()
}

fn pack_4bits_le(data: &[u8], last_byte: u8) -> Vec<u8> {
    let (chunks, remainder) = data.as_chunks::<2>();
    chunks
        .iter()
        .map(|a| (a[0] & 0xF) | (a[1] & 0xF) << 4)
        .chain(if remainder.is_empty() {
            None
        } else {
            Some(
                (remainder.get(0).copied().unwrap_or(last_byte) & 0xF)
                    | (remainder.get(1).copied().unwrap_or(last_byte >> 4) & 0xF) << 4,
            )
        })
        .collect()
}

fn pack_2bits_be(data: &[u8]) -> Vec<u8> {
    let (chunks, remainder) = data.as_chunks::<4>();
    chunks
        .iter()
        .map(|a| (a[0] & 0x3) << 6 | (a[1] & 0x3) << 4 | (a[2] & 0x3) << 2 | (a[3] & 0x3))
        .chain(if remainder.is_empty() {
            None
        } else {
            Some(
                (remainder.get(0).copied().unwrap_or(0) & 0x3) << 6
                    | (remainder.get(1).copied().unwrap_or(0) & 0x3) << 4
                    | (remainder.get(2).copied().unwrap_or(0) & 0x3) << 2
                    | (remainder.get(3).copied().unwrap_or(0) & 0x3),
            )
        })
        .collect()
}

fn pack_4bits_be(data: &[u8]) -> Vec<u8> {
    let (chunks, remainder) = data.as_chunks::<2>();
    chunks
        .iter()
        .map(|a| (a[0] & 0xF) << 4 | (a[1] & 0xF))
        .chain(if remainder.is_empty() {
            None
        } else {
            Some((remainder[0] & 0xF) << 4)
        })
        .collect()
}

fn unzip_palette<'a>(palette: impl IntoIterator<Item = &'a u16>) -> (Vec<[u8; 3]>, Vec<bool>) {
    palette
        .into_iter()
        .copied()
        .map(|x| (u16_to_color(x), u16_to_transparency(x)))
        .unzip()
}

fn get_color_index<'a>(color: &[u8; 3], palette: &[u16]) -> usize {
    let mut min_distance = MAX_COLOR_DISTANCE;
    let mut min_index = 0;
    for (index, palette_color) in palette.iter().enumerate() {
        let distance = color_distance(color, *palette_color);
        if distance == 0 {
            return index;
        }
        if distance < min_distance {
            min_distance = distance;
            min_index = index;
        }
    }
    min_index
}

const MAX_COLOR_DISTANCE: i32 = 31 * 3;

fn color_distance(color: &[u8; 3], palette_color: u16) -> i32 {
    let delta_r = ((color[0] >> 3) as i32) - (((palette_color >> 0) & 0x1F) as i32);
    let delta_g = ((color[1] >> 3) as i32) - (((palette_color >> 5) & 0x1F) as i32);
    let delta_b = ((color[2] >> 3) as i32) - (((palette_color >> 10) & 0x1F) as i32);
    return delta_r.abs() + delta_g.abs() + delta_b.abs();
}
