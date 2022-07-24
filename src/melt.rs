use anyhow::{anyhow, Context};
use argh::FromArgs;
use ck3save::FailedResolveStrategy;
use memmap::MmapOptions;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::File,
    io::{stdin, Read, Write},
    path::{Path, PathBuf},
    writeln,
};

/// Melt a binary encoded file into the plaintext equivalent.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "melt")]
pub(crate) struct MeltCommand {
    /// writes the melted contents to stdout instead of new file
    #[argh(switch, short = 'c')]
    to_stdout: bool,

    /// the behavior when an unknown binary key is encountered: ignore | stringify | error
    #[argh(option, short = 'u', default = "String::from(\"error\")")]
    unknown_key: String,

    /// specify the format of the input: eu4 | ck3 | hoi4 | rome
    #[argh(option)]
    format: Option<String>,

    /// output melted data to the given file
    #[argh(option, short = 'o')]
    out: Option<PathBuf>,

    /// retain binary properties in melted output
    #[argh(switch)]
    retain: bool,

    /// file to melt. Omission reads from stdin
    #[argh(positional)]
    file: Option<PathBuf>,
}

fn parse_failed_resolve(s: &str) -> anyhow::Result<FailedResolveStrategy> {
    match s {
        "ignore" => Ok(FailedResolveStrategy::Ignore),
        "stringify" => Ok(FailedResolveStrategy::Stringify),
        "error" => Ok(FailedResolveStrategy::Error),
        _ => Err(anyhow!("Unrecognized unknown key strategy")),
    }
}

enum MeltedDocument {
    Eu4(eu4save::MeltedDocument),
    Imperator(imperator_save::MeltedDocument),
    Hoi4(hoi4save::MeltedDocument),
    Ck3(ck3save::MeltedDocument),
}

impl MeltedDocument {
    pub fn data(&self) -> &[u8] {
        match self {
            MeltedDocument::Eu4(x) => x.data(),
            MeltedDocument::Imperator(x) => x.data(),
            MeltedDocument::Hoi4(x) => x.data(),
            MeltedDocument::Ck3(x) => x.data(),
        }
    }

    pub fn unknown_tokens(&self) -> &HashSet<u16> {
        match self {
            MeltedDocument::Eu4(x) => x.unknown_tokens(),
            MeltedDocument::Imperator(x) => x.unknown_tokens(),
            MeltedDocument::Hoi4(x) => x.unknown_tokens(),
            MeltedDocument::Ck3(x) => x.unknown_tokens(),
        }
    }
}

impl MeltCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        let format = self.format.as_deref().or_else(|| {
            self.file
                .as_deref()
                .and_then(|x| x.extension())
                .and_then(|x| x.to_str())
                .or_else(|| {
                    self.file
                        .as_deref()
                        .and_then(|x| x.file_name())
                        .and_then(|x| x.to_str())
                        .map(|x| x.trim_matches('.'))
                })
        });

        match format {
            Some(x) if x == "eu4" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let file = eu4save::Eu4File::from_slice(d)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                let binary = parsed_file.as_binary().context("not eu4 binary")?;
                let out = binary
                    .melter()
                    .on_failed_resolve(resolve)
                    .verbatim(self.retain)
                    .melt(&eu4save::EnvTokens)?;
                Ok(MeltedDocument::Eu4(out))
            }),
            Some(x) if x == "ck3" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let file = ck3save::Ck3File::from_slice(d)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                let binary = parsed_file.as_binary().context("not ck3 binary")?;
                let out = binary
                    .melter()
                    .on_failed_resolve(resolve)
                    .verbatim(self.retain)
                    .melt(&ck3save::EnvTokens)?;
                Ok(MeltedDocument::Ck3(out))
            }),
            Some(x) if x == "rome" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let file = imperator_save::ImperatorFile::from_slice(d)?;
                let mut zip_sink = Vec::new();
                let parsed_file = file.parse(&mut zip_sink)?;
                let binary = parsed_file.as_binary().context("not imperator binary")?;
                let out = binary
                    .melter()
                    .on_failed_resolve(resolve)
                    .verbatim(self.retain)
                    .melt(&imperator_save::EnvTokens)?;
                Ok(MeltedDocument::Imperator(out))
            }),
            Some(x) if x == "hoi4" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let file = hoi4save::Hoi4File::from_slice(d)?;
                let parsed_file = file.parse()?;
                let binary = parsed_file.as_binary().context("not hoi4 binary")?;
                let out = binary
                    .melter()
                    .on_failed_resolve(resolve)
                    .verbatim(self.retain)
                    .melt(&hoi4save::EnvTokens)?;
                Ok(MeltedDocument::Hoi4(out))
            }),
            _ => Err(anyhow!(
                "Unrecognized format: eu4, ck3, hoi4, and rome are supported"
            )),
        }
    }

    fn melt_game<F>(&self, f: F) -> anyhow::Result<i32>
    where
        F: Fn(&[u8]) -> anyhow::Result<MeltedDocument>,
    {
        let out = if let Some(path) = self.file.as_deref() {
            let in_file =
                File::open(&path).with_context(|| format!("Failed to open: {}", path.display()))?;
            let mmap = unsafe { MmapOptions::new().map(&in_file)? };
            f(&mmap[..])?
        } else {
            let sin = stdin();
            let mut reader = sin.lock();
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            f(&data[..])?
        };

        if let Some(out_path) = self.out.as_ref() {
            std::fs::write(out_path, out.data()).with_context(|| {
                format!("Unable to write to melted file: {}", out_path.display())
            })?;
        } else if self.to_stdout || self.file.is_none() {
            // Ignore write errors when writing to stdout so that one can pipe the output
            // to subsequent commands without fail
            let _ = std::io::stdout().write_all(out.data());
        } else {
            // Else we'll create a sibling file with a _melted suffix
            let out_path = melted_path(self.file.as_deref().unwrap());
            let mut out_file = File::create(&out_path)
                .with_context(|| format!("Failed to create melted file: {}", out_path.display()))?;

            out_file.write_all(out.data()).with_context(|| {
                format!("Failed to write to melted file: {}", out_path.display())
            })?;
        }

        let status = if out.unknown_tokens().is_empty() {
            0
        } else {
            1
        };
        for token in out.unknown_tokens() {
            let _ = writeln!(std::io::stderr(), "{:04x}", token);
        }

        Ok(status)
    }
}

fn melted_path<T: AsRef<Path>>(p: T) -> PathBuf {
    let path = p.as_ref();
    let in_name = path.file_stem().unwrap();
    let mut out_name = if path.extension().is_none() && in_name.to_string_lossy().starts_with('.') {
        let mut res = OsString::new();
        res.push("melted");
        res.push(in_name);
        res
    } else {
        let mut res = in_name.to_owned();
        res.push("_melted");
        res
    };

    if let Some(extension) = path.extension() {
        out_name.push(".");
        out_name.push(extension);
    }
    path.with_file_name(out_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn melt_path() {
        assert_eq!(
            melted_path("/tmp/a.eu4"),
            Path::new("/tmp").join("a_melted.eu4")
        );
        assert_eq!(
            melted_path("/tmp/gamestate"),
            Path::new("/tmp").join("gamestate_melted")
        );
    }
}
