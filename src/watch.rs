use anyhow::{anyhow, bail, Context};
use argh::FromArgs;
use ck3save::PdsDate;
use eu4save::file::{Eu4FileEntryName, Eu4FsFileKind};
use eu5save::Eu5Date;
use hoi4save::file::Hoi4FsFileKind;
use imperator_save::file::ImperatorFsFileKind;
use jomini::TextDeserializer;
use log::{debug, error, info, trace};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use vic3save::file::Vic3FsFileKind;

use crate::tokens::{
    ck3_tokens_resolver, eu4_tokens_resolver, eu5_tokens_resolver, hoi4_tokens_resolver,
    imperator_tokens_resolver, vic3_tokens_resolver,
};

/// Watch a save file for changes and create a copy with the save's date when changed
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "watch")]
pub(crate) struct WatchCommand {
    /// specify the format of the input: eu4 | eu5 | ck3 | hoi4 | rome | vic3
    /// if not specified, will be inferred from file extension
    #[argh(option)]
    format: Option<String>,

    /// output directory for saved copies
    /// if not specified, will use the same directory as the input file
    #[argh(option, short = 'o')]
    out_dir: Option<PathBuf>,

    /// frequency of snapshot creation. Options: 'daily' to create a snapshot on
    /// any date change, 'monthly' for each month, 'quarterly' for every three
    /// months, 'yearly' for annual snapshots, or 'decade' for snapshots when
    /// the decade changes (years ending in 0). If not specified, defaults are
    /// based on game type: EU4/CK3/Imperator=yearly, Victoria 3=quarterly,
    /// HOI4=monthly.
    #[argh(option)]
    frequency: Option<String>,

    /// file to watch for changes
    #[argh(positional)]
    file: PathBuf,
}

/// Frequency at which snapshots are taken
#[derive(Debug, PartialEq, Clone, Copy)]
enum SnapshotFrequency {
    /// Take a snapshot on any date change
    Daily,
    /// Take a snapshot only when the month changes
    Monthly,
    /// Take a snapshot every three months (quarters)
    Quarterly,
    /// Take a snapshot only when the year changes
    Yearly,
    /// Take a snapshot only when the decade changes (year % 10 == 0)
    Decade,
}

impl FromStr for SnapshotFrequency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" | "day" => Ok(SnapshotFrequency::Daily),
            "monthly" | "month" => Ok(SnapshotFrequency::Monthly),
            "quarterly" | "quarter" => Ok(SnapshotFrequency::Quarterly),
            "year" | "yearly" => Ok(SnapshotFrequency::Yearly),
            "decade" => Ok(SnapshotFrequency::Decade),
            _ => Err(anyhow!(
                "Unrecognized snapshot frequency. Use 'daily', 'monthly', 'quarterly', 'yearly', or 'decade'"
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
enum GameType {
    Eu4,
    Eu5,
    Ck3,
    Imperator,
    Vic3,
    Hoi4,
}

impl FromStr for GameType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eu4" => Ok(GameType::Eu4),
            "eu5" => Ok(GameType::Eu5),
            "ck3" => Ok(GameType::Ck3),
            "rome" => Ok(GameType::Imperator),
            "hoi4" => Ok(GameType::Hoi4),
            "v3" => Ok(GameType::Vic3),
            _ => Err(anyhow!(
                "Only eu4, eu5, ck3, vic3, hoi4, and imperator files supported"
            )),
        }
    }
}

impl GameType {
    fn default_frequency(&self) -> SnapshotFrequency {
        match self {
            GameType::Eu4 => SnapshotFrequency::Yearly,
            GameType::Eu5 => SnapshotFrequency::Yearly,
            GameType::Ck3 => SnapshotFrequency::Yearly,
            GameType::Imperator => SnapshotFrequency::Yearly,
            GameType::Vic3 => SnapshotFrequency::Quarterly,
            GameType::Hoi4 => SnapshotFrequency::Monthly,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
struct GameDate {
    year: i16,
    month: u8,
    day: u8,
}

impl GameDate {
    fn decade(&self) -> i16 {
        (self.year / 10) * 10
    }

    fn quarter(&self) -> u8 {
        (self.month - 1) / 3 + 1
    }

    fn should_snapshot(
        &self,
        last_snapshot: Option<&GameDate>,
        frequency: SnapshotFrequency,
    ) -> bool {
        match last_snapshot {
            None => true, // Always snapshot if no previous snapshot
            Some(last) => match frequency {
                SnapshotFrequency::Daily => true, // Always snapshot on any change
                SnapshotFrequency::Monthly => self.year != last.year || self.month != last.month,
                SnapshotFrequency::Quarterly => {
                    self.year != last.year || self.quarter() != last.quarter()
                }
                SnapshotFrequency::Yearly => self.year != last.year,
                SnapshotFrequency::Decade => self.decade() != last.decade(),
            },
        }
    }
}

impl Display for GameDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

struct SaveInfo {
    date: GameDate,
}

impl WatchCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp_secs()
            .format_target(false)
            .init();

        info!("Starting to watch file: {}", self.file.display());

        // Verify that the file exists before starting to watch
        if !self.file.exists() {
            bail!("File does not exist: {}", self.file.display());
        }

        let game_type = self.determine_game_type()?;

        // Parse the snapshot frequency or use the game-specific default
        let frequency = match &self.frequency {
            Some(freq_str) => freq_str.parse::<SnapshotFrequency>()?,
            None => {
                let default = game_type.default_frequency();
                info!("Using default frequency for {:?}: {:?}", game_type, default);
                default
            }
        };
        info!("Snapshot frequency: {:?}", frequency);

        let path = self.file.clone();

        // Get the parent directory of the file to watch
        let parent_dir = path
            .parent()
            .ok_or_else(|| anyhow!("Unable to determine parent directory of {}", path.display()))?;

        info!("Press Ctrl+C to stop watching");

        // Create a channel to receive the events
        let (tx, rx) = mpsc::channel();

        // Create a watcher with default configuration
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )?;

        // Start watching the parent directory for changes
        watcher.watch(parent_dir.as_ref(), RecursiveMode::NonRecursive)?;

        // Default output directory is subdirectory with the file stem name in the parent directory
        let out_dir = match &self.out_dir {
            Some(dir) => dir.clone(),
            None => {
                let parent = self.file.parent().unwrap_or_else(|| Path::new("."));
                let filename = self.file.file_stem().unwrap_or_default();
                let mut path = parent.to_path_buf();
                path.push(filename);

                path
            }
        };

        // Create the output directory if it doesn't exist
        if !out_dir.exists() {
            fs::create_dir_all(&out_dir).with_context(|| {
                format!("Failed to create output directory: {}", out_dir.display())
            })?;
        }

        info!("Output directory: {}", out_dir.display());

        // Track the last snapshot date for each game
        // Look for existing snapshots in the output directory when starting
        let start = Instant::now();
        let mut last_snapshot = self.find_latest_snapshot(&out_dir);
        if let Some(ref snapshot) = last_snapshot {
            let elapsed = start.elapsed();
            info!(
                "Starting from previous snapshot: {} [{}ms]",
                snapshot,
                elapsed.as_millis()
            );
        } else {
            let elapsed = start.elapsed();
            debug!(
                "No previous snapshots found in output directory [{}ms]",
                elapsed.as_millis()
            );
        }

        let mut ignore_next = false;

        // Set up Ctrl+C handler with an atomic flag
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            info!("Received Ctrl+C, shutting down gracefully...");
            r.store(false, Ordering::SeqCst);
        })
        .context("Error setting Ctrl+C handler")?;

        let debounce_timeout = Duration::from_millis(500);
        let mut last_event: Option<EventKind> = None;

        while running.load(Ordering::SeqCst) {
            // Try to receive an event with a short timeout to allow debounce checking
            match rx.recv_timeout(debounce_timeout) {
                Ok(event) => {
                    trace!("Received event: {:?}", event);
                    let EventKind::Modify(_) = event.kind else {
                        continue;
                    };

                    // Whenever we copy a file, we want to ignore the next event
                    // that comes in as it will be our event
                    if ignore_next {
                        debug!("Ignoring event due to previous copy operation");
                        ignore_next = false;
                        continue;
                    }

                    last_event = Some(event.kind);
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }

            if last_event.take().is_none() {
                continue;
            }

            // Process file and create snapshots only if we're still running
            if !running.load(Ordering::SeqCst) {
                break;
            }

            // Measure time taken to process the file
            let start = Instant::now();
            let save_info = match self.process_file(&game_type) {
                Ok(save_info) => {
                    let duration = start.elapsed();
                    info!(
                        "Detected file with date: {} [{}ms]",
                        save_info.date,
                        duration.as_millis()
                    );
                    save_info
                }
                Err(e) => {
                    let duration = start.elapsed();
                    error!("Error processing file: {} [{}ms]", e, duration.as_millis());
                    continue;
                }
            };

            if !save_info
                .date
                .should_snapshot(last_snapshot.as_ref(), frequency)
            {
                debug!(
                    "Skipping snapshot for date {}, waiting for next {} change",
                    save_info.date,
                    match frequency {
                        SnapshotFrequency::Daily => "date",
                        SnapshotFrequency::Monthly => "month",
                        SnapshotFrequency::Quarterly => "quarter",
                        SnapshotFrequency::Yearly => "year",
                        SnapshotFrequency::Decade => "decade",
                    }
                );
                continue;
            }

            let out_path = self.create_output_path(&save_info.date.to_string(), &out_dir);

            // Create parent directory if it doesn't exist
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        error!("Error creating directory {}: {}", parent.display(), e);
                        continue;
                    }
                }
            }

            let copy_start = Instant::now();
            if let Err(e) = fs::copy(&self.file, &out_path) {
                error!("Error copying file: {}", e);
            } else {
                let duration = copy_start.elapsed();
                info!(
                    "Successfully copied save to: {} [{}ms]",
                    out_path.display(),
                    duration.as_millis()
                );
                ignore_next = true;
                last_snapshot = Some(save_info.date);
            }
        }

        info!("Watch command completed");
        Ok(0)
    }

    fn process_file(&self, game_type: &GameType) -> anyhow::Result<SaveInfo> {
        let file = std::fs::File::open(&self.file)
            .with_context(|| format!("Failed to open file: {}", self.file.display()))?;

        // Parse the save to extract date (and make sure it is valid)
        let (year, month, day) = match game_type {
            GameType::Eu4 => {
                let file =
                    eu4save::Eu4File::from_file(file).context("Failed to parse EU4 save file")?;

                let meta = match file.kind() {
                    Eu4FsFileKind::Text(file) => {
                        let reader = jomini::text::TokenReader::new(file);
                        let mut deser = TextDeserializer::from_windows1252_reader(reader);
                        deser.deserialize::<eu4save::models::Meta>()?
                    }
                    Eu4FsFileKind::Binary(eu4_binary) => eu4_binary
                        .as_ref()
                        .deserializer(eu4_tokens_resolver())
                        .deserialize::<eu4save::models::Meta>()?,
                    Eu4FsFileKind::Zip(eu4_zip) => {
                        let mut entry = eu4_zip.get(Eu4FileEntryName::Meta)?;
                        entry.deserialize(eu4_tokens_resolver())?
                    }
                };

                (meta.date.year(), meta.date.month(), meta.date.day())
            }
            GameType::Eu5 => {
                #[derive(Debug, Deserialize)]
                pub struct ZipPrelude {
                    pub metadata: Metadata,
                }

                #[derive(Debug, Deserialize)]
                pub struct Metadata {
                    pub date: Eu5Date,
                }

                let file =
                    eu5save::Eu5File::from_file(file).context("Failed to parse EU5 save file")?;

                let prelude: ZipPrelude = match file.meta() {
                    eu5save::Eu5MetaKind::Text(mut txt) => {
                        ZipPrelude::deserialize(&mut txt.deserializer())
                    }
                    eu5save::Eu5MetaKind::Binary(mut bin) => {
                        ZipPrelude::deserialize(&mut bin.deserializer(&eu5_tokens_resolver()))
                    }
                }
                .context("Failed to parse EU5 metadata")?;

                let date = prelude.metadata.date;
                (date.year(), date.month(), date.day())
            }
            GameType::Ck3 => {
                let mut file =
                    ck3save::Ck3File::from_file(file).context("Failed to parse CK3 save file")?;

                let meta = match file.kind_mut() {
                    ck3save::file::Ck3FsFileKind::Text(file) => {
                        let reader = jomini::text::TokenReader::new(file);
                        let mut deser = TextDeserializer::from_utf8_reader(reader);
                        deser.deserialize::<ck3save::models::Header>()?
                    }
                    ck3save::file::Ck3FsFileKind::Binary(ck3_binary) => ck3_binary
                        .deserializer(ck3_tokens_resolver())
                        .deserialize::<ck3save::models::Header>()?,
                    ck3save::file::Ck3FsFileKind::Zip(ck3_zip) => {
                        let mut entry =
                            ck3_zip.meta().context("Failed to read metadata from zip")?;
                        entry
                            .deserializer(ck3_tokens_resolver())
                            .deserialize::<ck3save::models::Header>()?
                    }
                };

                (
                    meta.meta_data.meta_date.year(),
                    meta.meta_data.meta_date.month(),
                    meta.meta_data.meta_date.day(),
                )
            }
            GameType::Imperator => {
                let mut file = imperator_save::ImperatorFile::from_file(file)
                    .context("Failed to parse Imperator Rome save file")?;

                #[derive(Deserialize, Debug)]
                struct ImperatorMeta {
                    date: imperator_save::ImperatorDate,
                }

                let meta = match file.kind_mut() {
                    ImperatorFsFileKind::Text(file) => {
                        let reader = jomini::text::TokenReader::new(file);
                        let mut deser = TextDeserializer::from_utf8_reader(reader);
                        deser.deserialize::<ImperatorMeta>()?
                    }
                    ImperatorFsFileKind::Binary(imperator_binary) => imperator_binary
                        .deserializer(imperator_tokens_resolver())
                        .deserialize::<ImperatorMeta>()?,
                    ImperatorFsFileKind::Zip(imperator_zip) => imperator_zip
                        .meta()
                        .context("Failed to read metadata from zip")?
                        .deserializer(imperator_tokens_resolver())
                        .deserialize::<ImperatorMeta>()?,
                };

                (meta.date.year(), meta.date.month(), meta.date.day())
            }
            GameType::Vic3 => {
                let mut file = vic3save::Vic3File::from_file(file)
                    .context("Failed to parse Victoria 3 save file")?;

                #[derive(Deserialize, Debug)]
                struct Vic3MetaData {
                    meta_data: vic3save::savefile::MetaData,
                }

                let meta = match file.kind_mut() {
                    Vic3FsFileKind::Text(file) => {
                        let reader = jomini::text::TokenReader::new(file);
                        let mut deser = TextDeserializer::from_utf8_reader(reader);
                        deser.deserialize::<Vic3MetaData>()?
                    }
                    Vic3FsFileKind::Binary(vic3_binary) => vic3_binary
                        .deserializer(vic3_tokens_resolver())
                        .deserialize::<Vic3MetaData>()?,
                    Vic3FsFileKind::Zip(vic3_zip) => {
                        let mut entry = vic3_zip
                            .meta()
                            .context("Failed to read metadata from zip")?;
                        entry
                            .deserializer(vic3_tokens_resolver())
                            .deserialize::<Vic3MetaData>()?
                    }
                };

                (
                    meta.meta_data.game_date.year(),
                    meta.meta_data.game_date.month(),
                    meta.meta_data.game_date.day(),
                )
            }
            GameType::Hoi4 => {
                let mut file = hoi4save::Hoi4File::from_file(file)
                    .context("Failed to parse HOI4 save file")?;

                let meta = match file.kind_mut() {
                    Hoi4FsFileKind::Text(file) => {
                        let reader = jomini::text::TokenReader::new(file);
                        let mut deser = TextDeserializer::from_utf8_reader(reader);
                        deser.deserialize::<hoi4save::models::Hoi4Save>()?
                    }
                    Hoi4FsFileKind::Binary(hoi4_binary) => hoi4_binary
                        .as_ref()
                        .deserializer(hoi4_tokens_resolver())
                        .deserialize::<hoi4save::models::Hoi4Save>()?,
                };

                (meta.date.year(), meta.date.month(), meta.date.day())
            }
        };

        let game_date = GameDate { year, month, day };

        Ok(SaveInfo { date: game_date })
    }

    fn determine_game_type(&self) -> anyhow::Result<GameType> {
        if let Some(format) = &self.format {
            return format.parse();
        }

        let extension = self
            .file
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow!("Could not determine file format from extension"))?;

        extension
            .parse()
            .map_err(|_| anyhow!("Format of file unknown, please pass known format option"))
    }

    fn create_output_path(&self, date: &str, out_dir: &Path) -> PathBuf {
        let filename = self.file.file_stem().unwrap_or_default();
        let extension = self.file.extension().unwrap_or_default();

        let mut new_filename = filename.to_owned();
        new_filename.push("_");
        new_filename.push(date);

        let mut path = out_dir.to_path_buf();
        path.push(new_filename);

        if !extension.is_empty() {
            path.set_extension(extension);
        }

        path
    }

    fn find_snapshots(
        &self,
        out_dir: &Path,
    ) -> anyhow::Result<impl Iterator<Item = (PathBuf, GameDate)> + use<'_>> {
        let base_filename = self
            .file
            .file_stem()
            .expect("to have a file stem")
            .to_str()
            .expect("to convert filename to string");

        let entries = fs::read_dir(out_dir)?;
        let entries = entries.filter_map(Result::ok).filter_map(move |entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }

            let filename = path.file_stem()?.to_str()?;

            // Check if the filename starts with base_filename followed by underscore
            if !filename.starts_with(base_filename)
                || !filename[base_filename.len()..].starts_with('_')
            {
                return None;
            }

            // Extract date part (everything after base_name_)
            let date_part = &filename[base_filename.len() + 1..];

            // Try to parse the date in the format YYYY-MM-DD
            let mut parts = date_part.split('-');
            let year = parts.next()?.parse::<i16>().ok()?;
            let month = parts.next()?.parse::<u8>().ok()?;
            let day = parts.next()?.parse::<u8>().ok()?;

            Some((path, GameDate { year, month, day }))
        });
        Ok(entries)
    }

    fn find_latest_snapshot(&self, out_dir: &Path) -> Option<GameDate> {
        self.find_snapshots(out_dir)
            .ok()?
            .map(|(_, date)| date)
            .max()
    }
}
