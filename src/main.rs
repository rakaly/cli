mod cli;
mod melt;

fn main() -> anyhow::Result<()> {
    cli::run()
}
