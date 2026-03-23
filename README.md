# dearlystars_tool

## Patching instructions

1. Create a workspace directory and put the following inside it:
    - The dearlystars program (called `dearlystars` or `dearlystars.exe` on Windows).
    - The game rom (called `dearlystars.nds`).
    - The translation `.csv` files, all in the same subdirectory `translated_csv`.
      Note that the `.csv` filenames should all begin with `SCN_` or `F_SCN`, e.g. `SCN_AIH_A_MES.csv`.

2. In this workspace directory, extract the game text from the original game rom by running

    ```
    ./dearlystars extract-nds dearlystars.nds dearlystars_extracted
    ./dearlystars extract-bin -b dearlystars_extracted/data/F_SCN.BIN -i dearlystars_extracted/data/F_SCN.IDX F_SCN
    ```

3. Inject the text from the translation files into the game files by running

    ```
    ./dearlystars inject-bbq-text translated_csv F_SCN
    ```

4. Build the translated game rom by running

    ```
    ./dearlystars build-bin F_SCN -b dearlystars_extracted/data/F_SCN.BIN -i dearlystars_extracted/data/F_SCN.IDX
    ./dearlystars build-nds dearlystars_extracted dearlystars_translated.nds
    ```

    Note that the first of these commands can take several minutes!
