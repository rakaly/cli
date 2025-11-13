mod cli;
mod interpolation;
mod json;
mod melt;
mod tokens;
mod watch;

fn main() {
    std::process::exit(match cli::run() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("{:?}", e);
            2
        }
    });
}
