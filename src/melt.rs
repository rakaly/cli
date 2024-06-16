use anyhow::{anyhow, bail, Context};
use argh::FromArgs;
use ck3save::FailedResolveStrategy;
use memmap::MmapOptions;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::File,
    io::{self, stdin, BufWriter, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
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

    /// specify the format of the input: eu4 | ck3 | hoi4 | rome | vic3
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
    Vic3(vic3save::MeltedDocument),
}

impl MeltedDocument {
    pub fn unknown_tokens(&self) -> &HashSet<u16> {
        match self {
            MeltedDocument::Eu4(x) => x.unknown_tokens(),
            MeltedDocument::Imperator(x) => x.unknown_tokens(),
            MeltedDocument::Hoi4(x) => x.unknown_tokens(),
            MeltedDocument::Ck3(x) => x.unknown_tokens(),
            MeltedDocument::Vic3(x) => x.unknown_tokens(),
        }
    }
}

struct MelterOptions {
    retain: bool,
    resolve: FailedResolveStrategy,
}

struct Melter {
    options: MelterOptions,
    kind: MelterKind,
}

enum MelterKind {
    Eu4,
    Ck3,
    Imperator,
    Vic3,
    Hoi4,
}

impl FromStr for MelterKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eu4" => Ok(MelterKind::Eu4),
            "ck3" => Ok(MelterKind::Ck3),
            "rome" => Ok(MelterKind::Imperator),
            "hoi4" => Ok(MelterKind::Hoi4),
            "v3" => Ok(MelterKind::Vic3),
            _ => bail!("Only eu4, ck3, vic3, hoi4, and imperator files supported"),
        }
    }
}

impl Melter {
    pub fn melt<W: Write>(&mut self, data: &[u8], writer: W) -> anyhow::Result<MeltedDocument> {
        match self.kind {
            MelterKind::Eu4 => {
                let file = eu4save::Eu4File::from_slice(data)?;
                let out = file
                    .melter()
                    .on_failed_resolve(self.options.resolve)
                    .verbatim(self.options.retain)
                    .melt(writer, &eu4save::EnvTokens)?;
                Ok(MeltedDocument::Eu4(out))
            }
            MelterKind::Ck3 => {
                let file = ck3save::Ck3File::from_slice(data)?;
                let out = file
                    .melter()
                    .on_failed_resolve(self.options.resolve)
                    .verbatim(self.options.retain)
                    .melt(writer, &ck3save::EnvTokens)?;
                Ok(MeltedDocument::Ck3(out))
            }
            MelterKind::Imperator => {
                let file = imperator_save::ImperatorFile::from_slice(data)?;
                let out = file
                    .melter()
                    .on_failed_resolve(self.options.resolve)
                    .verbatim(self.options.retain)
                    .melt(writer, &imperator_save::EnvTokens)?;
                Ok(MeltedDocument::Imperator(out))
            }
            MelterKind::Vic3 => {
                let file = vic3save::Vic3File::from_slice(data)?;
                let out = file
                    .melter()
                    .on_failed_resolve(self.options.resolve)
                    .verbatim(self.options.retain)
                    .melt(writer, &vic3save::EnvTokens)?;
                Ok(MeltedDocument::Vic3(out))
            }
            MelterKind::Hoi4 => {
                let file = hoi4save::Hoi4File::from_slice(data)?;
                let out = file
                    .melter()
                    .on_failed_resolve(self.options.resolve)
                    .verbatim(self.options.retain)
                    .melt(writer, &hoi4save::EnvTokens)?;
                Ok(MeltedDocument::Hoi4(out))
            }
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

        let melter_kind = format
            .context("Format of file unknown, please pass format option")?
            .parse::<MelterKind>()?;

        let options = MelterOptions {
            retain: self.retain,
            resolve: parse_failed_resolve(self.unknown_key.as_str())?,
        };

        let mut melter = Melter {
            kind: melter_kind,
            options,
        };

        let input: Box<dyn AsRef<[u8]>> = if let Some(path) = self.file.as_deref() {
            let in_file =
                File::open(path).with_context(|| format!("Failed to open: {}", path.display()))?;
            let mmap = unsafe { MmapOptions::new().map(&in_file)? };
            Box::new(mmap)
        } else {
            let mut buf = Vec::new();
            stdin().read_to_end(&mut buf)?;
            Box::new(buf)
        };

        let out = if let Some(out_path) = self.out.as_ref() {
            let out_file = File::create(out_path)
                .with_context(|| format!("Unable to create melted file: {}", out_path.display()))?;
            let writer = BufWriter::with_capacity(32 * 1024, out_file);
            Some(melter.melt(input.as_ref().as_ref(), writer)?)
        } else if self.to_stdout || self.file.is_none() {
            let out = std::io::stdout();
            let lock = out.lock();
            let writer = BufWriter::with_capacity(32 * 1024, lock);
            let result = melter.melt(input.as_ref().as_ref(), writer);
            match result {
                Ok(x) => Some(x),

                // Ignore io errors when writing to stdout so that one can pipe the output
                // to subsequent commands without fail
                Err(e) => match e.chain().find_map(|ie| ie.downcast_ref::<io::Error>()) {
                    Some(io_err) if matches!(io_err.kind(), std::io::ErrorKind::BrokenPipe) => None,
                    _ => bail!(e),
                },
            }
        } else {
            // Else we'll create a sibling file with a _melted suffix
            let out_path = melted_path(self.file.as_deref().unwrap());
            let out_file = File::create(&out_path)
                .with_context(|| format!("Failed to create melted file: {}", out_path.display()))?;
            let writer = BufWriter::with_capacity(32 * 1024, out_file);
            Some(melter.melt(input.as_ref().as_ref(), writer)?)
        };

        let status = match &out {
            Some(melted) if melted.unknown_tokens().is_empty() => 0,
            None => 0,
            _ => 1,
        };

        for token in out.iter().flat_map(|x| x.unknown_tokens().iter()) {
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
