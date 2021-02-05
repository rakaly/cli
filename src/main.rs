mod cli;
mod config;
mod melt;
mod upload;
mod upload_client;
mod log;

fn main() {
    std::process::exit(match cli::run() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("{:?}", e);
            2
        }
    });
}
