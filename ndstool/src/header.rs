use binrw::binrw;

#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DsHeader {
    pub title: [u8; 0xC],
    pub gamecode: [u8; 0x4],
    pub makercode: [u8; 0x2],
    pub unitcode: u8,          // product code. 0=NDS, 2=NDS+DSi, 3=DSi
    pub devicetype: u8,        // device code. 0 = normal
    pub devicecap: u8,         // device size. (1<<n Mbit)
    pub reserved_a: [u8; 0x7], // 0x015..0x01D
    pub dsi_flags: u8,
    pub nds_region: u8,
    pub romversion: u8,
    pub autostart: u8,        // 0x01F
    pub arm9_rom_offset: u32, // points to libsyscall and rest of ARM9 binary
    pub arm9_entry_address: u32,
    pub arm9_ram_address: u32,
    pub arm9_size: u32,
    pub arm7_rom_offset: u32,
    pub arm7_entry_address: u32,
    pub arm7_ram_address: u32,
    pub arm7_size: u32,
    pub fnt_offset: u32,
    pub fnt_size: u32,
    pub fat_offset: u32,
    pub fat_size: u32,
    pub arm9_overlay_offset: u32,
    pub arm9_overlay_size: u32,
    pub arm7_overlay_offset: u32,
    pub arm7_overlay_size: u32,
    pub rom_control_info1: u32, // 0x00416657 for OneTimePROM
    pub rom_control_info2: u32, // 0x081808F8 for OneTimePROM
    pub banner_offset: u32,
    pub secure_area_crc: u16,
    pub rom_control_info3: u16,              // 0x0D7E for OneTimePROM
    pub arm9_autoload_hook_ram_address: u32, // magic1 (64 bit encrypted magic code to disable LFSR)
    pub arm7_autoload_hook_ram_address: u32, // magic2
    pub secure_area_disable: [u8; 0x8],      // unique ID for homebrew
    pub application_end_offset: u32,         // rom size
    pub rom_header_size: u32,
    pub arm9_parameters_table_offset: u32, // reserved... ?
    pub arm7_parameters_table_offset: u32,
    pub dsi_ntr_rom_region_end: u16,
    pub dsi_twl_rom_region_start: u16,
    pub reserved_b: [u8; 0x2C],
    pub logo: [u8; 0x9C],
    pub logo_crc: u16,
    pub header_crc: u16,

    // 0x160..0x17F reserved
    pub debug_rom_offset: u32,
    pub debug_size: u32,
    pub debug_ram_address: u32,
    pub offset_0x16c: u32,
    pub zero: [u8; 0x10],
}

#[binrw]
#[brw(little)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DsiExtraFields {
    pub global_mbk_setting: [u8; 0x14],
    pub arm9_mbk_setting: [u32; 0x3],
    pub arm7_mbk_setting: [u32; 0x3],
    pub mbk9_wramcnt_setting: u32,

    pub region_flags: u32,
    pub access_control: u32,
    pub scfg_ext_mask: u32,
    pub reserved_c: [u8; 0x3],
    pub appflags: u8,

    pub dsi9_rom_offset: u32,
    pub reserved_d: u32,
    pub dsi9_ram_address: u32,
    pub dsi9_size: u32,
    pub dsi7_rom_offset: u32,
    pub device_list_ram_address: u32,
    pub dsi7_ram_address: u32,
    pub dsi7_size: u32,

    pub digest_ntr_start: u32,
    pub digest_ntr_size: u32,
    pub digest_twl_start: u32,
    pub digest_twl_size: u32,
    pub sector_hashtable_start: u32,
    pub sector_hashtable_size: u32,
    pub block_hashtable_start: u32,
    pub block_hashtable_size: u32,
    pub digest_sector_size: u32,
    pub digest_block_sectorcount: u32,

    pub banner_size: u32,
    pub shared_20000_size: u8,
    pub shared_20001_size: u8,
    pub eula_version: u8,
    pub use_ratings: u8,
    pub total_rom_size: u32,
    pub shared_20002_size: u8,
    pub shared_20003_size: u8,
    pub shared_20004_size: u8,
    pub shared_20005_size: u8,
    pub arm9i_parameters_table_offset: u32,
    pub arm7i_parameters_table_offset: u32,

    pub modcrypt1_start: u32,
    pub modcrypt1_size: u32,
    pub modcrypt2_start: u32,
    pub modcrypt2_size: u32,

    pub tid_low: u32,
    pub tid_high: u32,
    pub public_sav_size: u32,
    pub private_sav_size: u32,
    pub reserved_e: [u8; 0xB0],
    pub age_ratings: [u8; 0x10],

    pub hmac_arm9: [u8; 0x14],
    pub hmac_arm7: [u8; 0x14],
    pub hmac_digest_master: [u8; 0x14],
    pub hmac_icon_title: [u8; 0x14],
    pub hmac_arm9i: [u8; 0x14],
    pub hmac_arm7i: [u8; 0x14],
    pub crypto_reserved_a: [u8; 0x14],
    pub crypto_reserved_b: [u8; 0x14],
    pub hmac_arm9_no_secure: [u8; 0x14],
    pub crypto_reserved_c: [u8; 0xA4C],
    pub debug_args: [u8; 0x180],
    pub rsa_signature: [u8; 0x80],
}
