use argh::FromArgs;

/// Rakaly server commands exposed locally
#[derive(FromArgs, PartialEq, Debug)]
struct RakalyCommand {
    /// print the version and exit
    #[argh(switch)]
    version: bool,

    #[argh(subcommand)]
    cmd: Option<GameCommand>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum GameCommand {
    Melt(crate::melt::MeltCommand),
    Upload(crate::upload::UploadCommand),
}

pub fn run() -> anyhow::Result<i32> {
    let args: RakalyCommand = argh::from_env();
    if args.version {
        println!(env!("CARGO_PKG_VERSION"));
        Ok(0)
    } else if let Some(cmd) = args.cmd {
        match cmd {
            GameCommand::Melt(melt) => melt.exec(),
            GameCommand::Upload(upload) => upload.exec(),
        }
    } else {
        println!("execute --help to see available options");
        Ok(0)
    }
}
