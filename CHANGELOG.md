## v0.3.3 - 2021-04-27

- Update melting logic 
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
