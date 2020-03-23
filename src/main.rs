use structopt::StructOpt;

use membuffer_extract::{process, Config};

const BUF_CAP_BYTES: usize = 1 << 12;

fn main() -> std::io::Result<()> {
    let config = Config::from_args();
    eprintln!("{:?}", config);

    process(&config, BUF_CAP_BYTES)?;

    Ok(())
}
