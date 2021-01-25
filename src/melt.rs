use anyhow::{anyhow, Context};
use argh::FromArgs;
use jomini::FailedResolveStrategy;
use memmap::MmapOptions;
use std::{collections::HashSet, fs::File, io::Write, path::PathBuf, writeln};

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

    /// specify the format of the input: eu4 | ck3 | rome
    #[argh(option)]
    format: Option<String>,

    /// file to melt
    #[argh(positional)]
    file: PathBuf,
}

fn parse_failed_resolve(s: &str) -> anyhow::Result<FailedResolveStrategy> {
    match s {
        "ignore" => Ok(FailedResolveStrategy::Ignore),
        "stringify" => Ok(FailedResolveStrategy::Stringify),
        "error" => Ok(FailedResolveStrategy::Error),
        _ => Err(anyhow!("Unrecognized unknown key strategy")),
    }
}

impl MeltCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        let format = self
            .format
            .as_deref()
            .or_else(|| self.file.extension().and_then(|x| x.to_str()));

        match format {
            Some(x) if x == "eu4" => {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                self.melt_game(|d| Ok(eu4save::melt(d, resolve)?))
            }
            Some(x) if x == "ck3" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let out = ck3save::Melter::new()
                    .with_on_failed_resolve(resolve)
                    .melt(d)?;
                Ok(out)
            }),
            Some(x) if x == "rome" => self.melt_game(|d| {
                let resolve = parse_failed_resolve(self.unknown_key.as_str())?;
                let out = imperator_save::Melter::new()
                    .with_on_failed_resolve(resolve)
                    .melt(d)?;
                Ok(out)
            }),
            _ => Err(anyhow!(
                "Unrecognized format: eu4, ck3, and rome are supported"
            )),
        }
    }

    fn melt_game<F>(&self, f: F) -> anyhow::Result<i32>
    where
        F: Fn(&[u8]) -> anyhow::Result<(Vec<u8>, HashSet<u16>)>,
    {
        let path = self.file.as_path();

        let in_file =
            File::open(path).with_context(|| format!("Failed to open: {}", path.display()))?;
        let mmap = unsafe { MmapOptions::new().map(&in_file)? };
        let (out, tokens) = f(&mmap[..])?;

        if self.to_stdout {
            // Ignore write errors when writing to stdout so that one can pipe the output
            // to subsequent commands without fail
            let _ = std::io::stdout().write_all(&out[..]);
        } else {
            // Else we'll create a sibling file with a _melted suffix
            let in_extension = path.extension().unwrap();
            let in_name = path.file_stem().unwrap();
            let mut out_name = in_name.to_owned();
            out_name.push("_melted.");
            out_name.push(in_extension);
            let out_path = path.with_file_name(out_name);

            let mut out_file = File::create(&out_path)
                .with_context(|| format!("Failed to create melted file: {}", out_path.display()))?;

            out_file.write_all(&out[..]).with_context(|| {
                format!("Failed to write to melted file: {}", out_path.display())
            })?;
        }

        let status = if tokens.is_empty() { 0 } else { 1 };
        for token in &tokens {
            let _ = writeln!(std::io::stderr(), "{:04x}", token);
        }

        Ok(status)
    }
}
