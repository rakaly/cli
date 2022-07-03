mod cli;
mod config;
mod json;
mod log;
mod melt;
mod upload;
mod upload_client;

fn main() {
    std::process::exit(match cli::run() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("{:?}", e);
            2
        }
    });
}
