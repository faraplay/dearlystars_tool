# dearlystars_tool

## Patching instructions

1. First download the translation `.csv` files and put them all in the same folder, e.g. called `translated_csv`.
   The `.csv` filenames should all begin with `SCN_` or `F_SCN`, e.g. `SCN_AIH_A_MES.csv`.

2. Extract the game text from the original game rom by running

    ```
    dearlystars extract-nds dearlystars.nds dearlystars
    dearlystars extract-bin -b dearlystars/data/F_SCN.BIN -i dearlystars/data/F_SCN.IDX F_SCN
    ```

3. Inject the text from the translation files into the game files by running

    ```
    dearlystars inject-bbq-text translated_csv F_SCN
    ```

4. Build the translated game rom by running

    ```
    dearlystars build-bin F_SCN -b dearlystars/data/F_SCN.BIN -i dearlystars/data/F_SCN.IDX
    dearlystars build-nds dearlystars dearlystars_translated.nds
    ```

    Note that the first of these commands can take several minutes!
