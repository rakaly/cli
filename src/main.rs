mod cli;
mod json;
mod melt;
mod tokens;

fn main() {
    std::process::exit(match cli::run() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("{:?}", e);
            2
        }
    });
}
