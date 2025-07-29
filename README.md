![ci](https://github.com/rakaly/cli/workflows/ci/badge.svg)

# Rakaly CLI

The Rakaly CLI provides a convenient way of accessing [PDX.Tools](https://pdx.tools/) functionality locally.

## Features

- ✔ Supports converting (melting) EU4, CK3, HOI4, and Imperator Rome saves to their plaintext equivalent
- ✔ Cross platform: run rakaly-cli on mac, windows, and linux 
- ✔ Lightweight: Small executable that can be download and ran -- no dependencies 

## Install

 - Go to the [latest releases](https://github.com/rakaly/cli/releases/latest)
 - Download desired build (mac, windows, or linux)
 - Extract
 - (optional): Add to your computers path for easier access
   - For windows, PATH can be modified through [the GUI](https://www.architectryan.com/2018/03/17/add-to-the-path-on-windows-10/) or [the terminal](https://stackoverflow.com/q/9546324/433785)
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

#### Variable Interpolation

The `json` command supports variable interpolation.

```bash
rakaly json --interpolation --format utf8 config.txt
```

Input:

```
@scale = 10
position = { x = @[scale * 2] y = @scale }
```

Output:

```json
{"position":{"x":20,"y":10}}
```

### Watch and archive saves

The `rakaly watch` command continuously monitors a save file and creates snapshots to maintain a historical record of the campaign at desired intervals.

When there is file activity (eg, when the game saves), the command parses the save to extract the current in-game date. The command compares the current save's date to the date of the last snapshot taken (if any). If the criteria for the frequency are met (eg, the year changed for yearly frequency), it creates a copy of the modified save file in the specified output directory.

For instance, start an ironman EU4 campaign named "hsa" and run the watch command:

```bash
rakaly watch hsa.eu4 
```

And soon enough, an "hsa" directory will be created and start to fill with saves as EU4 autosaves roughly every three months:

- `hsa/hsa_1444-11-11.eu4`
- `hsa/hsa_1445-03-01.eu4`
- `hsa/hsa_1446-03-01.eu4`

In the above example, we only see yearly snapshots, as that is the default snapshot interval the `watch` command uses for EU4 saves. EU4's first autosave in 1445 was in March, hence the snapshot date of 1445-03-01. Note that EU4 does not have consistent autosave dates between sessions.

The snapshot interval can be customized:

- daily
- monthly (default for HOI4)
- quarterly (default for Vic3)
- yearly (default for EU4, CK3, and Imperator)
- decade

Note that these snapshot intervals may differ from how frequently the game autosaves.

When started, the command scans the output directory for existing snapshots related to the input file. It identifies the most recent snapshot date and will only create new snapshots for in-game dates after that one, according to the frequency rules. This prevents duplicate snapshots if you restart the watcher.

The watch command is intended to be minimal. For a fully featured save game manager, see [pdxu](https://github.com/crschnick/pdx_unlimiter).
