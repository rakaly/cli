use anyhow::anyhow;
use argh::FromArgs;
use ck3save::Ck3Melt;
use eu4save::{file::Eu4ParsedText, Eu4File};
use eu5save::Eu5Melt;
use hoi4save::{file::Hoi4ParsedText, Hoi4File};
use imperator_save::ImperatorMelt;
use jomini::{
    envelope::{JominiFileKind, SaveDataKind},
    json::{DuplicateKeyMode, JsonOptions},
    TextTape,
};
use std::{
    io::{BufWriter, Cursor},
    path::PathBuf,
};
use vic3save::Vic3Melt;

use crate::{
    interpolation::InterpolatedTape,
    tokens::{
        ck3_tokens_resolver, eu4_tokens_resolver, eu5_tokens_resolver, hoi4_tokens_resolver,
        imperator_tokens_resolver, vic3_tokens_resolver,
    },
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

    /// perform variable interpolation and convert exists operators to equals (requires --format)
    #[argh(switch)]
    interpolation: bool,

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

#[derive(Clone, Copy)]
pub enum Encoding {
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
        // Validate that interpolation flag is only used with generic files (not game files)
        if self.interpolation {
            let extension = self.file.extension().and_then(|x| x.to_str());
            if matches!(
                extension,
                Some("eu4") | Some("ck3") | Some("rome") | Some("hoi4") | Some("v3")
            ) {
                return Err(anyhow!("--interpolation flag can only be used with generic files (not game-specific file extensions), requires --format"));
            }
        }
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
            Some("eu5" | "ck3" | "rome" | "v3") => {
                let file = jomini::envelope::JominiFile::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                match extension {
                    Some("eu5") => {
                        let options = eu5save::MeltOptions::new()
                            .on_failed_resolve(strategy)
                            .verbatim(verbatim);
                        let resolver = eu5save::SaveResolver::from_file(&file, eu5_tokens_resolver())?;
                        Eu5Melt::melt(&mut (&file), options, &resolver, &mut out)?;
                    }
                    Some("ck3") => {
                        let options = ck3save::MeltOptions::new()
                            .on_failed_resolve(strategy)
                            .verbatim(verbatim);
                        Ck3Melt::melt(&mut (&file), options, ck3_tokens_resolver(), &mut out)?;
                    }
                    Some("rome") => {
                        let options = imperator_save::MeltOptions::new()
                            .on_failed_resolve(strategy)
                            .verbatim(verbatim);
                        ImperatorMelt::melt(
                            &mut (&file),
                            options,
                            imperator_tokens_resolver(),
                            &mut out,
                        )?;
                    }
                    Some("v3") => {
                        let options = vic3save::MeltOptions::new()
                            .on_failed_resolve(strategy)
                            .verbatim(verbatim);
                        Vic3Melt::melt(&mut (&file), options, vic3_tokens_resolver(), &mut out)?;
                    }
                    _ => unreachable!(),
                }

                let file = jomini::envelope::JominiFile::from_slice(out.get_ref())?;
                let JominiFileKind::Uncompressed(SaveDataKind::Text(txt)) = file.kind() else {
                    return Err(anyhow!("Unexpected file kind after melting"));
                };

                let all = txt.body().get_ref().get_ref().as_slice();
                let body = &all[txt.body().content_offset() as usize..];
                let tape = TextTape::from_slice(body)?;
                tape.utf8_reader()
                    .json()
                    .with_options(options)
                    .to_writer(writer)
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
            _ => {
                let encoding = parse_encoding(&self.format)?;

                if self.interpolation {
                    let tape = jomini::TextTape::from_slice(&data)?;
                    let interpolated_tape =
                        InterpolatedTape::from_tape_with_interpolation(&tape, encoding)
                            .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                    interpolated_tape.to_writer_with_options(writer, options, encoding)
                } else {
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
            }
        };

        Ok(0)
    }
}
