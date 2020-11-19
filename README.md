![ci](https://github.com/rakaly/cli/workflows/ci/badge.svg)

# Rakaly CLI

The Rakaly CLI provides a convenient way of accessing [Rakaly](https://rakaly.com/eu4/) functionality locally.

## Features

- ✔ Supports converting (melting) EU4, CK3, and Imperator Rome saves to their plaintext equivalent
- ✔ Cross platform: run rakaly-cli on mac, windows, and linux 
- ✔ Lightweight: Small executable that can be download and ran -- no dependencies 

## Install

 - Go to the [latest releases](https://github.com/nickbabcock/rrrocket/releases/latest)
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

The above example will create a plaintext `aq_melted.eu4` file that one can open up and inspect in a text editor. Moreover, this melted save may be continued in EU4 as if it was a normal game (CK3 and Imperator Rome remain untested in this aspect). 

The melt command determines how to interpret the save file by looking at the extension (`.eu4`, `.rome`, or `.ck3`).

If outputting to stdout is more your style:

```plain
rakaly melt --to-stdout aq.eu4
```

Whenever there is a content patch for the supported games, the rakaly-cli will be out of date until the next update. The default behavior of the melt command is to fail when unexpected tokens from the new content is encountered. To make the melt command perserve 
