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
