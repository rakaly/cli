use anyhow::anyhow;
use argh::FromArgs;
use ck3save::{file::Ck3ParsedText, Ck3File};
use eu4save::{file::Eu4ParsedText, Eu4File};
use eu5save::Eu5File;
use hoi4save::{file::Hoi4ParsedText, Hoi4File};
use imperator_save::{file::ImperatorParsedText, ImperatorFile};
use jomini::{
    json::{DuplicateKeyMode, JsonOptions},
    TextTape,
};
use std::{
    io::{BufWriter, Cursor},
    path::PathBuf,
};
use vic3save::{file::Vic3ParsedText, Vic3File};

use crate::tokens::{
    ck3_tokens_resolver, eu4_tokens_resolver, eu5_tokens_resolver, hoi4_tokens_resolver,
    imperator_tokens_resolver, vic3_tokens_resolver,
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
        let stdout = std::io::stdout();
        let writer = BufWriter::new(stdout.lock());

        let _ = match extension {
            Some("eu4") => {
                let file = Eu4File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if file.encoding().is_binary() || file.encoding().is_zip() {
                    let options = eu4save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, eu4_tokens_resolver(), &mut out)?;
                    Eu4ParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    Eu4ParsedText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some("eu5") => {
                let file = Eu5File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let melted = if file.header().kind().is_binary() {
                    let options = eu5save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, eu5_tokens_resolver(), &mut out)?;
                    true
                } else {
                    false
                };

                let tape_data = if melted {
                    out.get_ref().as_slice()
                } else {
                    // For text files, we need to skip the header
                    &data[file.header().header_len()..]
                };

                let tape = TextTape::from_slice(tape_data)?;
                tape.utf8_reader()
                    .json()
                    .with_options(options)
                    .to_writer(writer)
            }
            Some("ck3") => {
                let file = Ck3File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), ck3save::Encoding::Text) {
                    let options = ck3save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, ck3_tokens_resolver(), &mut out)?;
                    Ck3ParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    Ck3ParsedText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some("rome") => {
                let file = ImperatorFile::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), imperator_save::Encoding::Text) {
                    let options = imperator_save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, imperator_tokens_resolver(), &mut out)?;
                    ImperatorParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    ImperatorParsedText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some("hoi4") => {
                let file = Hoi4File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), hoi4save::Encoding::Plaintext) {
                    let options = hoi4save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, hoi4_tokens_resolver(), &mut out)?;
                    Hoi4ParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    Hoi4ParsedText::from_slice(&data)?
                };

                text.reader().json().with_options(options).to_writer(writer)
            }
            Some("v3") => {
                let file = Vic3File::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                let text = if !matches!(file.encoding(), vic3save::Encoding::Text) {
                    let options = vic3save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, vic3_tokens_resolver(), &mut out)?;
                    Vic3ParsedText::from_slice(out.get_ref().as_slice())?
                } else {
                    Vic3ParsedText::from_slice(&data)?
                };

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
