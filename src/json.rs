use anyhow::{anyhow, Context};
use argh::FromArgs;
use ck3save::{
    file::{Ck3ParsedFileKind, Ck3Text},
    Ck3File,
};
use eu4save::{
    file::{Eu4ParsedFileKind, Eu4Text},
    Eu4File,
};
use hoi4save::{
    file::{Hoi4ParsedFileKind, Hoi4Text},
    Hoi4File,
};
use imperator_save::{
    file::{ImperatorParsedFileKind, ImperatorText},
    ImperatorFile,
};
use jomini::{
    json::{DuplicateKeyMode, JsonOptions},
    TextTape,
};
use std::path::PathBuf;
use vic3save::{
    file::{Vic3ParsedFileKind, Vic3Text},
    Vic3File,
};

/// convert save and game files to json
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "json")]
pub(crate) struct JsonCommand {
    /// specify the format of the input: utf-8 | windows-1252
    #[argh(option, short = 'f', default = "String::from(\"windows-1252\")")]
    format: String,

    /// specify how to handle duplicate keys: preserve | group | key-value-pairs
    #[argh(option, short = 'k', default = "String::from(\"preserve\")")]
    duplicate_keys: String,

    /// pretty-print json
    #[argh(switch)]
    pretty: bool,

    /// file to melt. Omission reads from stdin
    #[argh(positional)]
    file: PathBuf,
}

fn parse_duplicate_keys(s: &str) -> anyhow::Result<DuplicateKeyMode> {
    match s.to_lowercase().as_str() {
        "preserve" => Ok(DuplicateKeyMode::Preserve),
        "group" => Ok(DuplicateKeyMode::Group),
        "key-value-pairs" => Ok(DuplicateKeyMode::KeyValuePairs),
        _ => Err(anyhow!("Unrecognized duplicate key option")),
    }
}

enum Encoding {
    Utf8,
    Windows1252,
}

fn parse_encoding(s: &str) -> anyhow::Result<Encoding> {
    match s.to_lowercase().as_str() {
        "utf-8" => Ok(Encoding::Utf8),
        "windows-1252" => Ok(Encoding::Windows1252),
        _ => Err(anyhow!("Unrecognized encoding option")),
    }
}

impl JsonCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        let extension = self.file.extension().and_then(|x| x.to_str());
        let data = std::fs::read(&self.file)?;
        let keys = parse_duplicate_keys(&self.duplicate_keys)?;
        let options = JsonOptions::new()
            .with_prettyprint(self.pretty)
            .with_duplicate_keys(keys);

        let verbatim = true;
        let strategy = jomini::binary::FailedResolveStrategy::Ignore;

        let _ = match extension {
            Some(x) if x == "eu4" => {
                let file = Eu4File::from_slice(&data)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                match parsed_file.kind() {
                    Eu4ParsedFileKind::Text(text) => text
                        .reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    Eu4ParsedFileKind::Binary(binary) => {
                        let melted = binary
                            .melter()
                            .verbatim(verbatim)
                            .on_failed_resolve(strategy)
                            .melt(&eu4save::EnvTokens)?;
                        Eu4Text::from_slice(melted.data())
                            .and_then(|x| x.parse())
                            .context("unable to parse melted eu4 output")?
                            .reader()
                            .json()
                            .with_options(options)
                            .to_writer(std::io::stdout())
                    }
                }
            }
            Some(x) if x == "ck3" => {
                let file = Ck3File::from_slice(&data)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                match parsed_file.kind() {
                    Ck3ParsedFileKind::Text(text) => text
                        .reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    Ck3ParsedFileKind::Binary(binary) => {
                        let melted = binary
                            .melter()
                            .verbatim(verbatim)
                            .on_failed_resolve(strategy)
                            .melt(&ck3save::EnvTokens)?;
                        Ck3Text::from_slice(melted.data())
                            .context("unable to parse melted ck3 output")?
                            .reader()
                            .json()
                            .with_options(options)
                            .to_writer(std::io::stdout())
                    }
                }
            }
            Some(x) if x == "rome" => {
                let file = ImperatorFile::from_slice(&data)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                match parsed_file.kind() {
                    ImperatorParsedFileKind::Text(text) => text
                        .reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    ImperatorParsedFileKind::Binary(binary) => {
                        let melted = binary
                            .melter()
                            .verbatim(verbatim)
                            .on_failed_resolve(strategy)
                            .melt(&imperator_save::EnvTokens)?;
                        ImperatorText::from_slice(melted.data())
                            .context("unable to parse melted imperator output")?
                            .reader()
                            .json()
                            .with_options(options)
                            .to_writer(std::io::stdout())
                    }
                }
            }
            Some(x) if x == "hoi4" => {
                let file = Hoi4File::from_slice(&data)?;
                let parsed_file = file.parse()?;
                match parsed_file.kind() {
                    Hoi4ParsedFileKind::Text(text) => text
                        .reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    Hoi4ParsedFileKind::Binary(binary) => {
                        let melted = binary
                            .melter()
                            .verbatim(verbatim)
                            .on_failed_resolve(strategy)
                            .melt(&hoi4save::EnvTokens)?;
                        Hoi4Text::from_slice(melted.data())
                            .context("unable to parse melted hoi4 output")?
                            .reader()
                            .json()
                            .with_options(options)
                            .to_writer(std::io::stdout())
                    }
                }
            }
            Some(x) if x == "v3" => {
                let file = Vic3File::from_slice(&data)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                match parsed_file.kind() {
                    Vic3ParsedFileKind::Text(text) => text
                        .reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    Vic3ParsedFileKind::Binary(binary) => {
                        let melted = binary
                            .melter()
                            .verbatim(verbatim)
                            .on_failed_resolve(strategy)
                            .melt(&vic3save::EnvTokens)?;
                        Vic3Text::from_slice(melted.data())
                            .context("unable to parse melted vic3 output")?
                            .reader()
                            .json()
                            .with_options(options)
                            .to_writer(std::io::stdout())
                    }
                }
            }
            _ => {
                let encoding = parse_encoding(&self.format)?;
                let tape = TextTape::from_slice(&data)?;
                match encoding {
                    Encoding::Utf8 => tape
                        .utf8_reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                    Encoding::Windows1252 => tape
                        .windows1252_reader()
                        .json()
                        .with_options(options)
                        .to_writer(std::io::stdout()),
                }
            }
        };

        Ok(0)
    }
}
