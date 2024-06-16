use anyhow::anyhow;
use argh::FromArgs;
use ck3save::{file::Ck3Text, Ck3File};
use eu4save::{file::Eu4ParsedText, Eu4File};
use hoi4save::{file::Hoi4Text, Hoi4File};
use imperator_save::{file::ImperatorText, ImperatorFile};
use jomini::{
    json::{DuplicateKeyMode, JsonOptions},
    TextTape,
};
use std::{
    io::{BufWriter, Cursor},
    path::PathBuf,
};
use vic3save::Vic3File;

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
        let stdout = std::io::stdout();
        let writer = BufWriter::new(stdout.lock());

        let _ = match extension {
            Some(x) if x == "eu4" => {
                let file = Eu4File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if file.encoding().is_binary() || file.encoding().is_zip() {
                    file.melter()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim)
                        .melt(&mut out, &eu4save::EnvTokens)?;
                    Eu4ParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    Eu4ParsedText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some(x) if x == "ck3" => {
                let file = Ck3File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), ck3save::Encoding::Text) {
                    file.melter()
                        .verbatim(true)
                        .melt(&mut out, &ck3save::EnvTokens)?;
                    Ck3Text::from_slice(out.get_ref())?
                } else {
                    Ck3Text::from_slice(&data)?
                };
                text.reader().json().with_options(options).to_writer(writer)
            }
            Some(x) if x == "rome" => {
                let file = ImperatorFile::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), imperator_save::Encoding::Text) {
                    file.melter()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim)
                        .melt(&mut out, &imperator_save::EnvTokens)?;
                    ImperatorText::from_slice(out.get_ref().as_slice())?
                } else {
                    ImperatorText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some(x) if x == "hoi4" => {
                let file = Hoi4File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), hoi4save::Encoding::Plaintext) {
                    file.melter()
                        .verbatim(true)
                        .melt(&mut out, &hoi4save::EnvTokens)?;
                    Hoi4Text::from_slice(out.get_ref())?
                } else {
                    Hoi4Text::from_slice(&data)?
                };
                text.reader().json().with_options(options).to_writer(writer)
            }
            Some(x) if x == "v3" => {
                let file = Vic3File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                file.melter()
                    .verbatim(true)
                    .melt(&mut out, &vic3save::EnvTokens)?;
                let text = Hoi4Text::from_slice(out.get_ref())?;
                text.reader().json().with_options(options).to_writer(writer)
            }
            _ => {
                let encoding = parse_encoding(&self.format)?;
                let tape = TextTape::from_slice(&data)?;
                match encoding {
                    Encoding::Utf8 => tape
                        .utf8_reader()
                        .json()
                        .with_options(options)
                        .to_writer(writer),
                    Encoding::Windows1252 => tape
                        .windows1252_reader()
                        .json()
                        .with_options(options)
                        .to_writer(writer),
                }
            }
        };

        Ok(0)
    }
}
