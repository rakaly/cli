## v0.8.7 - 2025-12-06

- Update to support melting EU5 1.0.10 saves
- Improved melting support of HOI4 1.17 saves

## v0.8.6 - 2025-11-28

- Fix missing lookup values in EU5 1.0.8 saves

## v0.8.5 - 2025-11-28

- Update to support EU5 1.0.8 saves
- Fix incorrect melting output for "resistance" fields in EU5

## v0.8.4 - 2025-11-24

- ~~Update to support EU5 1.0.8 saves~~ (this proved too optimistic)

## v0.8.3 - 2025-11-24

- Update to support HOI4 1.17 saves
- Performance improvements to all operations working over binary / ironman saves.

## v0.8.2 - 2025-11-12

- Add the `--interpolate` flag for variable interpolation on the json command
- Organize watched EU5 saves by playthrough name instead of file name

## v0.8.1 - 2025-11-07

- Strip ironman flag on melted eu5 saves to avoid reverting to ironman 

## v0.8.0 - 2025-11-05

- Add EU5 support
- Update to support CK3 1.18

## v0.7.1 - 2025-10-05

- Update to support Vic3 1.10.4 saves

## v0.7.0 - 2025-10-02

- (breaking change): JSON output update
- Update to support Vic3 1.10 saves
- Update to support CK3 1.17 saves

## v0.6.1 - 2025-06-20

- Update to support Vic3 1.9 saves

## v0.6.0 - 2025-05-02

- Add `watch` command that will monitor a save file and create a copy at desired in-game date intervals

## v0.5.7 - 2025-04-28

- Update to support CK3 1.16.0 saves

## v0.5.6 - 2025-03-27

- Update to support CK3 1.15.0 saves

## v0.5.5 - 2025-03-12

- Update to support HOI4 1.16.0 saves
- Update to support CK3 1.14.3 saves

## v0.5.4 - 2024-11-21

- Update to support melting Vic3 1.8 saves
- Update to support melting HOI4 1.15 saves

## v0.5.3 - 2024-10-04

- Update to support CK3 1.13 saves

## v0.5.2 - 2024-07-05

- Fix regression when melting Vic3, Imperator, and CK3 text zip save files

## v0.5.1 - 2024-07-02

- All games now use a constant memory melter
- Update to support VIC3 1.7 saves

## v0.5.0 - 2024-05-08

- Remove underutilized `upload` command to mitigate false positive virus reports
- Update to support EU4 1.37 saves

## v0.4.25 - 2024-04-12

- Release native mac m-series binaries

## v0.4.24 - 2024-03-11

- Update to support HOI4 1.14 saves
- Fix ironman ck3 json generation

## v0.4.23 - 2024-03-07

- Update to support CK3 1.12 saves
- Update to support VIC3 1.6 saves

## v0.4.22 - 2023-11-21

- Improved support for VIC3 1.5 saves

## v0.4.21 - 2023-11-15

- Update to support CK3 1.11 saves
- Update to support VIC3 1.5 saves

## v0.4.20 - 2023-11-06

- Update to support EU4 1.36 saves

## v0.4.19 - 2023-10-15

- Update to support HOI4 1.13 saves

## v0.4.18 - 2023-10-09

- Update to support VIC3 1.4.2 tokens

## v0.4.17 - 2023-08-29

- Update to support VIC3 1.4 tokens
- Update to support CK3 1.10 tokens

## v0.4.16 - 2023-07-10

- Additional accuracy for melted vic3 output
- Update to latest upload API 

## v0.4.15 - 2023-05-26

- Improve vic3 melt accuracy for pop_statistsics

## v0.4.14 - 2023-05-25

- Improved accuracy of vic3 melted numbers

## v0.4.13 - 2023-05-23

- Update vic3 tokens to 1.3

## v0.4.12 - 2023-05-14

- Update hoi4 melter to exclude ironman key from output

## v0.4.11 - 2023-05-12

- Update to support EU4 1.35.3 tokens
- Update to support CK3 1.9 tokens

## v0.4.10 - 2023-04-18

- Update to support EU4 1.35 tokens

## v0.4.9 - 2023-03-13

- Update to HOI4 1.12.11 tokens
- Update to Vic3 1.2 tokens
- Improve HOI4 melting accuracy

## v0.4.8 - 2022-12-17

- Update to Ck3 1.8 tokens

## v0.4.7 - 2022-12-05

- Update to Vic3 1.1 tokens

## v0.4.6 - 2022-11-23

- Fix vic3 dates in melted output
   - Properly detect `1.1.1` as a date when encoded as 43808760
   - Encode `real_date` as a known date as it falls outside the heuristic range

## v0.4.5 - 2022-11-07

- Add Vic3 support

## v0.4.4 - 2022-09-29

- Update to HOI4 1.12 tokens

## v0.4.3 - 2022-09-22

- Fix incorrect CK3 1.7 melted format for floats

## v0.4.2 - 2022-09-12

- Update to CK3 1.7 tokens
- Update to EU4 1.34 tokens
- Performance improvements when melting zipped saves

## v0.4.1 - 2022-07-24

- Fix incorrect binary tokens for HOI4, Imperator, and CK3.

## v0.4.0 - 2022-07-03

- Add initial support for the `json` subcommand, which will convert game and
save files (including binary ones) into JSON output on stdout. CLI arguments are
subject to change.

## v0.3.19 - 2022-06-01

- Support CK3 1.6 saves

## v0.3.18 - 2022-03-20

- Update EU4 melted output to be compatible with loading the save from the in game menu by not containing a terminating newline

## v0.3.17 - 2021-03-06

- Support HOI4 1.11.8 saves

## v0.3.16 - 2021-02-22

- Support CK3 1.5 saves
- Support EU4 1.33 saves

## v0.3.15 - 2021-11-24

- Update tokens to support new HOI4 1.11 additions
- Detect and melt known HOI4 dates correctly

## v0.3.14 - 2021-11-14

- Update tokens to support new EU4 1.32 additions

## v0.3.13 - 2021-09-24

- Up to 15% performance improvement when melting saves
- Uploaded saves now take advantage of Rakaly's new storage format

## v0.3.12 - 2021-07-25

- Support melting dates that have a zero year
- Support melting files that are missing a file name (eg: `.eu4`)

## v0.3.11 - 2021-07-10

- Fix HOI4 saves not melting correctly when `operatives` is present and decoded incorrectly.

## v0.3.10 - 2021-07-04

- Fix improper melted output when a name ended with a quote

## v0.3.9 - 2021-06-08

- EU4 dates prior to 5000 BC can now be melted properly and not cause an error
- EU4 dates that would cause an error going forward are ignored unless `--unknown-key error` is provided

## v0.3.8 - 2021-05-29

- Fix obscenely large CK3 melted output (introduced in v0.3.7) due to not accounting for hidden objects
- Fix some array values not being properly indented

## v0.3.7 - 2021-05-28

- Fix missing HOI4 binary tokens in linux build
- Melt with tabs instead of spaces
- Melted quoted values are now escaped as needed. A quoted value that contained quotes didn't have the inner quotes escaped, leading to output that could fail to parse.

## v0.3.6 - 2021-05-18

- Add new `--retain` flag that will not rewrite melted output to conform more to plaintext properties
- Melted output now only uses newlines for line endings
- eu4: correct number of decimal points are always used
- eu4: fixed the possibility of melted ids being detected as dates
- ck3: rewrite save header line with new metadata size
- ck3: omit certain ironman fields (`ironman` and `ironman_manager`) from melted output

## v0.3.5 - 2021-05-03

- Update tokens to support EU4 1.31.2
- Increase accuracy for melted EU4 64bit floats by up to a 10,000th
- Significant update to CK3 melting output:
  - Fix melted output containing quotes when plaintext has no quotes
  - Rewrite save header to declare the melted output is uncompressed plaintext
  - Increase accuracy of decoding 64 bit floats (alternative format) from ironman format
  - Write numbers as integers when ignoring the fractional component would not result in a loss of accuracy just like the plaintext format
  - Identified additional tokens that use the alternative float format
  - Fixed more numbers being interpreted as dates

## v0.3.4 - 2021-04-29

- Update tokens to support EU4 1.31.1
- Fix regression introduced in v0.8.4 where ck3 and imperator would melt all numbers as dates

## v0.3.3 - 2021-04-27

- Update melting to more accurately decode 64 bit floats (in rare cases large positive numbers could be interpreted as negative)
- Update melting to support Eu4 Leviathan prehistoric dates
- Update melting to support alternative Ck3 floating point format 
- Update tokens to support Eu4 Leviathan

## v0.3.2 - 2021-03-16

- Update CK3 tokens to support the 1.3 update

## v0.3.1 - 2021-02-17

- Update Imperator tokens to support the 2.0 update

## v0.3.0 - 2021-02-09

- Added a new `upload` command so one can upload saves to Rakaly

## v0.2.2 - 2021-02-05

- Fix minor defects in previous release assets (eg: include version string and don't package tempory directory).

## v0.2.1 - 2021-02-05

- Improved melting support. Won't quote values that aren't quoted in plaintext

## v0.2.0 - 2021-02-01

- Initial HOI4 melting support
- Updated ck3 tokens

## v0.1.2 - 2021-01-26

- Update latest tokens for ck3 and eu4
- Specify out file with -o/--out
- Fix bug that didn't allow creating a melted file from an extensionless file

## v0.1.1 - 2021-01-25

- Update melting logic to correctly melt seeds
- Return exit code 1 when unknown tokens are encountered when they are stringified into the output
- Print unknown tokens encountered to stderr

## v0.1.0 - 2021-01-13

Include a `--format` flag to the melter to dictate how the file should be decoded

## v0.0.10 - 2020-12-06

Negative binary dates are properly melted into their plaintext equivalent.
