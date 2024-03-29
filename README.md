![ci](https://github.com/rakaly/cli/workflows/ci/badge.svg)

# Rakaly CLI

The Rakaly CLI provides a convenient way of accessing [Rakaly](https://rakaly.com/eu4/) functionality locally.

## Features

- ✔ Supports converting (melting) EU4, CK3, HOI4, and Imperator Rome saves to their plaintext equivalent
- ✔ Cross platform: run rakaly-cli on mac, windows, and linux 
- ✔ Lightweight: Small executable that can be download and ran -- no dependencies 

## Install

 - Go to the [latest releases](https://github.com/rakaly/cli/releases/latest)
 - Download desired build (mac, windows, or linux)
 - Extract
 - (optional): Add to your computers path for easier access
 - Enjoy

## Documentation

### Melting Save Files

Rakaly CLI can convert binary encoded saves to their plaintext equivalent in a process called melting.

```plain
rakaly melt aq.eu4
```

The above example will create a plaintext `aq_melted.eu4` file that one can open up and inspect in a text editor. Moreover, this melted save may be continued in EU4 as if it was a normal game (the other games remain untested in this aspect). 

The melt command determines how to interpret the save file by looking at the extension (`.eu4`, `.rome`, `.hoi4`, or `.ck3`).

If outputting to stdout is more your style:

```plain
rakaly melt --to-stdout aq.eu4
```

Whenever there is a content patch for the supported games, the rakaly-cli will be out of date until the next update. The default behavior of the melt command is to fail when unexpected tokens from the new content is encountered. To make the melt command perserve through the tokens and encode them as hexadecimals in the output:

```plain
rakaly melt --unknown-key stringify aq.eu4
```

When unknown tokens are encountered with the stringify strategy then the unknown tokens are printed to stderr and the exit code is 1.

The melter knows how to melt a given file based on its file extension. In the event this heuristic is incorrect, one can explicitly provide the desired format:

```plain
rakaly melt --format eu4 --to-stdout gamestate
```

### Conversion to JSON

The `json` subcommand will convert game and save files (including binary ones) into JSON output on stdout.

```bash
rakaly json aq.eu4
```

The output can be pretty printed:

```bash
rakaly json --pretty aq.eu4
```

By default, duplicate keys are preserved in the JSON, but this can be configured:

```bash
rakaly json --duplicate-keys preserve  aq.eu4
rakaly json --duplicate-keys group aq.eu4
rakaly json --duplicate-keys key-value-pairs aq.eu4
```

When converting game files, pass the character encoding so that non-ascii characters are represented correctly:

```bash
rakaly json --format windows-1252 achievements.txt
```

### Uploading Save Files

One can upload EU4 saves to Rakaly with:

```bash
rakaly upload --user $RAKALY_USER --api-key $RAKALY_API_KEY ./path/to/save.eu4
```

To avoid supplying the credential arguments every time, one can create a config file (let's say `config.toml`):

```toml
user = "100"
api_key = "not-a-real-api-key"
```

And reference it like so

```
rakaly upload --config config.toml ./path/to/save.eu4
```
