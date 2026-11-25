use anyhow::anyhow;
use argh::FromArgs;
use ck3save::Ck3Melt;
use eu4save::{flavor::Eu4Flavor, Eu4File};
use eu5save::Eu5Melt;
use hoi4save::Hoi4File;
use imperator_save::ImperatorMelt;
use jomini::{
    envelope::{JominiFileKind, SaveDataKind},
    json::{DuplicateKeyMode, JsonOptions},
    text::syntax::{parse, parse_with, Hoi4Flavor, Parse},
    Utf8Encoding, Windows1252Encoding,
};
use std::{
    borrow::Cow,
    io::{BufWriter, Cursor, Write},
    path::{Path, PathBuf},
};
use vic3save::Vic3Melt;

use crate::{
    interpolation::{check_syntax, interpolate, load_variables},
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

    /// file whose variables are available to the input file (requires
    /// --interpolation). Repeatable, and read in the order given.
    #[argh(option)]
    variables: Vec<PathBuf>,

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

impl jomini::Encoding for Encoding {
    fn decode<'a>(&self, data: &'a [u8]) -> Cow<'a, str> {
        match self {
            Encoding::Utf8 => Utf8Encoding::decode(data),
            Encoding::Windows1252 => Windows1252Encoding::decode(data),
        }
    }
}

fn parse_encoding(s: &str) -> anyhow::Result<Encoding> {
    match s.to_lowercase().as_str() {
        "utf-8" => Ok(Encoding::Utf8),
        "windows-1252" => Ok(Encoding::Windows1252),
        _ => Err(anyhow!("Unrecognized encoding option")),
    }
}

/// The syntax parser panics on a file that a `u32` can't index
pub fn check_size(data: &[u8]) -> anyhow::Result<()> {
    if data.len() as u64 > u64::from(u32::MAX) {
        return Err(anyhow!("text is 4 GiB or larger"));
    }
    Ok(())
}

/// Print the recoverable syntax errors of `parsed` and fail on the others.
fn report_syntax(parsed: &Parse, name: &str) -> anyhow::Result<()> {
    let data = parsed.tree().source();
    let warnings = check_syntax(data, parsed.errors()).map_err(|e| anyhow!("{}:{}", name, e))?;
    for warning in warnings {
        eprintln!("warning: {}:{}", name, warning);
    }
    Ok(())
}

/// Drop the `EU4txt` or `HOI4txt` header
fn strip_header<'a>(data: &'a [u8], header: &str) -> anyhow::Result<&'a [u8]> {
    data.strip_prefix(header.as_bytes())
        .ok_or_else(|| anyhow!("Expected text to start with {}", header))
}

/// Positions in melted text are not positions in the save file
fn text_name(file: &Path, melted: bool) -> String {
    let suffix = if melted { " (melted)" } else { "" };
    format!("{}{}", file.display(), suffix)
}

impl JsonCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        // Validate that interpolation flag is only used with generic files (not game files)
        if self.interpolation {
            let extension = self.file.extension().and_then(|x| x.to_str());
            if matches!(
                extension,
                Some("eu4") | Some("eu5") | Some("ck3") | Some("rome") | Some("hoi4") | Some("v3")
            ) {
                return Err(anyhow!("--interpolation flag can only be used with generic files (not game-specific file extensions), requires --format"));
            }
        }
        if !self.variables.is_empty() && !self.interpolation {
            return Err(anyhow!("--variables requires --interpolation"));
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
        let mut writer = BufWriter::new(stdout.lock());

        let written = match extension {
            Some("eu4") => {
                let file = Eu4File::from_slice(&data)?;
                let melted = file.encoding().is_binary() || file.encoding().is_zip();
                let mut out = Vec::new();
                let text = if melted {
                    let options = eu4save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, eu4_tokens_resolver(), &mut out)?;
                    out.as_slice()
                } else {
                    data.as_slice()
                };

                let body = strip_header(text, "EU4txt")?;
                check_size(body)?;
                let parsed = parse(body);
                report_syntax(&parsed, &text_name(&self.file, melted))?;
                parsed
                    .document(Eu4Flavor::new())
                    .json()
                    .with_options(options)
                    .to_writer(&mut writer)
            }
            Some("eu5" | "ck3" | "rome" | "v3") => {
                let file = jomini::envelope::JominiFile::from_slice(&data)?;
                let mut out = Cursor::new(Vec::new());
                match extension {
                    Some("eu5") => {
                        let options = eu5save::MeltOptions::new()
                            .on_failed_resolve(strategy)
                            .verbatim(verbatim);
                        let resolver =
                            eu5save::SaveResolver::from_file(&file, eu5_tokens_resolver())?;
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
                check_size(body)?;
                let parsed = parse(body);
                report_syntax(&parsed, &text_name(&self.file, true))?;
                parsed
                    .document(Utf8Encoding::new())
                    .json()
                    .with_options(options)
                    .to_writer(&mut writer)
            }
            Some("hoi4") => {
                let file = Hoi4File::from_slice(&data)?;
                let melted = !matches!(file.encoding(), hoi4save::Encoding::Plaintext);
                let mut out = Vec::new();
                let text = if melted {
                    let options = hoi4save::MeltOptions::new()
                        .on_failed_resolve(strategy)
                        .verbatim(verbatim);
                    file.melt(options, hoi4_tokens_resolver(), &mut out)?;
                    out.as_slice()
                } else {
                    data.as_slice()
                };

                let body = strip_header(text, "HOI4txt")?;
                check_size(body)?;
                let parsed = parse_with(body, &Hoi4Flavor);
                report_syntax(&parsed, &text_name(&self.file, melted))?;
                parsed
                    .document(Utf8Encoding::new())
                    .json()
                    .with_options(options)
                    .to_writer(&mut writer)
            }
            _ => {
                let encoding = parse_encoding(&self.format)?;

                check_size(&data)?;
                let parsed = parse(&data);
                report_syntax(&parsed, &text_name(&self.file, false))?;

                if self.interpolation {
                    let (scope, warnings) = load_variables(&self.variables, encoding)?;
                    for warning in &warnings {
                        eprintln!("warning: {}", warning);
                    }
                    let interpolated = interpolate(&parsed, &scope, encoding)?;
                    for warning in &interpolated.warnings {
                        eprintln!("warning: {}:{}", self.file.display(), warning);
                    }
                    interpolated
                        .block
                        .json()
                        .with_options(options)
                        .to_writer(&mut writer)
                } else {
                    parsed
                        .document(encoding)
                        .json()
                        .with_options(options)
                        .to_writer(&mut writer)
                }
            }
        };

        // `to_writer` does not flush, and a `BufWriter` drop discards errors
        let written = written.and_then(|()| writer.flush());

        // A closed pipe, as with `rakaly json ... | head`, is not a failure
        match written {
            Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => Err(e.into()),
            _ => Ok(0),
        }
    }
}
