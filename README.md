# dearlystars_tool

## Patching instructions

1. Create a workspace directory and put the following inside it:
    - The dearlystars program (called `dearlystars` or `dearlystars.exe` on Windows).
    - The game rom (called `dearlystars.nds`).
    - The SCN translation `.csv` files, all in the same subdirectory `translated_csv`.
      Note that the `.csv` filenames should all begin with `SCN_` or `F_SCN`, e.g. `SCN_AIH_A_MES.csv`.
    - The TBL translation file `F_TBL.csv`, which should also be placed in the `translated_csv` subdirectory
    - The translated images, saved as `.png` files, all in the same subdirectory `translated_png`.
    - The executable strings `.csv` file (called `arm9_overlay_strings.csv`).

2. In this workspace directory, extract the game files from the original game rom by running

    ```
    ./dearlystars extract-nds dearlystars.nds dearlystars_extracted
    ```

3. Inject the text from the SCN translation files in `translated_csv` into the game SCN file by running

    ```
    ./dearlystars extract-bin -b dearlystars_extracted/data/F_SCN.BIN -i dearlystars_extracted/data/F_SCN.IDX F_SCN
    ./dearlystars inject-bbq-text translated_csv F_SCN
    ./dearlystars build-bin F_SCN -b dearlystars_extracted/data/F_SCN.BIN -i dearlystars_extracted/data/F_SCN.IDX
    ```

    Note that the `build-bin` command can take several minutes!

4. Inject the text from the TBL translation file in `translated_csv` into the game SCN file by running

    ```
    ./dearlystars extract-bin -b dearlystars_extracted/data/F_TBL.BIN -i dearlystars_extracted/data/F_TBL.IDX F_TBL
    ./dearlystars inject-bbq-text translated_csv F_TBL
    ./dearlystars build-bin F_TBL -b dearlystars_extracted/data/F_TBL.BIN -i dearlystars_extracted/data/F_TBL.IDX
    ```

5. Inject the translated images in `translated_png` into the game AGL file by running

    ```
    ./dearlystars extract-bin -b dearlystars_extracted/data/F_AGL.BIN -i dearlystars_extracted/data/F_AGL.IDX F_AGL
    ./dearlystars inject-gld-images translated_png F_AGL/AGL -p injected_preview
    ./dearlystars build-bin F_AGL -b dearlystars_extracted/data/F_AGL.BIN -i dearlystars_extracted/data/F_AGL.IDX
    ```

    Due to the game files using palettes to store images, the colors of the images may
    be changed to match the ingame palette. Previews of the injected images with changed colors
    are written to the folder `injected_preview`.

6. Inject the translated strings from `arm9_overlay_strings.csv` into the extracted game code by running

    ```
    ./dearlystars inject-exec-strings arm9_overlay_strings.csv dearlystars_extracted
    ```

7. Build the translated game rom by running

    ```
    ./dearlystars build-nds dearlystars_extracted dearlystars_translated.nds
    ```

## Acknowledgements

Code for the `ndstool` module is heavily based on [the `ndstool` program](https://github.com/devkitPro/ndstool)
and the very detailed documentation [GBATEK](https://problemkaputt.de/gbatek.htm).

Details of the GLD image format were figured out by **HARMAp**

Code for executable string injection (in `dearlystars/src/arm9overlay.rs`) was written by **airkingneo** (though some edits have since been made).
