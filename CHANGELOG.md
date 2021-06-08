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
