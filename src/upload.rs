use crate::{
    config::{default_base_url, default_config_path, read_config},
    log::configure_logger,
    upload_client::UploadClient,
};
use anyhow::anyhow;
use argh::FromArgs;
use std::path::PathBuf;

/// Upload a save file to Rakaly
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "upload")]
pub(crate) struct UploadCommand {
    /// rakaly user id
    #[argh(option, short = 'u')]
    user: Option<String>,

    /// rakaly api key
    #[argh(option)]
    api_key: Option<String>,

    /// path to rakaly config
    #[argh(option, short = 'c')]
    config: Option<PathBuf>,

    /// increase the verbosity of the command.
    #[argh(switch, short = 'v')]
    verbose: u8,

    /// file to upload
    #[argh(positional)]
    file: PathBuf,
}

impl UploadCommand {
    pub(crate) fn exec(&self) -> anyhow::Result<i32> {
        configure_logger(self.verbose)?;

        let config = self.config.clone().or_else(default_config_path);
        log::debug!("rakaly config file path: {:?}", config);
        let config = config.map(read_config).transpose()?;

        let user = self
            .user
            .as_deref()
            .or_else(|| config.as_ref().map(|x| x.user.as_str()));

        let api_key = self
            .api_key
            .as_deref()
            .or_else(|| config.as_ref().map(|x| x.api_key.as_str()));

        let base_url = config
            .as_ref()
            .map(|x| x.base_url.clone())
            .unwrap_or_else(default_base_url);

        let user = user.ok_or_else(|| anyhow!("user must be supplied via cli or config"))?;
        let api_key =
            api_key.ok_or_else(|| anyhow!("api_key must be supplied via cli or config"))?;

        let client = UploadClient {
            user,
            api_key,
            base_url: base_url.as_str(),
        };

        let path = self.file.as_path();
        let new_save = client.upload(path)?;
        println!("{}", &new_save.save_id);
        println!("{}/eu4/saves/{}", &base_url, &new_save.save_id);

        if !new_save.used_save_slot {
            println!(
                "save slot was not used, {} remaining",
                new_save.remaining_save_slots
            );
        } else {
            println!(
                "save slot was used, {} remaining",
                new_save.remaining_save_slots
            );
        }
        Ok(0)
    }
}
