## Version 0.5.0

Add string injection into executables (`arm9.bin` and overlays).

Adds:

- New command `inject-exec-strings` to inject strings into game executable files.

## Version 0.4.1

Changes:

- More accurate `blz` compression algorithm.
- Arm9 and overlay executable sizes are now written to the `.nds` rom in more places, meaning that rebuilt `.nds` roms with edited executables can boot.
- Removed the `-d` and `-c` options from the `extract-nds` and `build-nds` commands, the program now always automatically decompresses/compresses executable files.

## Version 0.4.0

Add compression/decompression for executables (`arm9.bin` and overlays) in `.nds` files.

Adds:

- New commands `decompress-blz` and `compress-blz` to decompress/compress executable files.
- New option `-d` for the `extract-nds` command, this automatically decompresses executable files when extracting.
- New option `-c` for the `build-nds` command, this automatically compresses executable files when rebuilding.

Fixes:

- Fixed gld image injection for sprite formats 2 and 3.
- Correct header size is now used for non-DSi roms when rebuilding.

## Version 0.3.1

Changes:

- Changed BBQ message insertion to use new column format with a translated speaker name column.
- BBQ text extraction now outputs comma-separated values instead of tab-separated values.

Fixes:

- Black pixels are no longer turned into transparent pixels during GLD image injection.

## Version 0.3.0

Add basic GLD image extraction and injection.

Changes:

- Add GLD image extraction and injection functionality.

## Version 0.2.0

Improve accuracy of the `.nds` rom built by `ndstool`.

Changes:

- The DSi binaries `arm9i.bin` and `arm7i.bin` are now automatically decrypted when extracting a DSi `.nds` file
- Digest hashes are now calculated when building a DSi `.nds` file

## Version 0.1.0

Initial release.

Features:

- Extracting and rebuilding `.nds` files
- Extracting and rebuilding `.bin/.idx` archives
- Converting `.bbq` files to/from `.yaml` files
- Extracting text from and injecting text into `.bbq` files
