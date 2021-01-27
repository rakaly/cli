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
