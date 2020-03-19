use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;

use console::Term;

use fxhash::FxBuildHasher;

use indicatif::HumanBytes;

use structopt::StructOpt;

use trace::Trace;

mod trace;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "membuffer-extract",
    about = "Extracts compressed membuffer output."
)]
struct Config {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

fn main() -> std::io::Result<()> {
    let config = Config::from_args();
    println!("{:?}", config);

    let file = std::fs::File::open(config.file)?;
    let trace = Trace::new(BufReader::new(file));

    let mut huge_pages: HashMap<_, _, FxBuildHasher> = HashMap::default();

    let term = Term::stdout();
    term.write_line(&format!(
        "Processed (decompressed): {}",
        HumanBytes(trace.so_far())
    ))?;

    for (chunk, so_far) in trace {
        for addr in chunk.into_iter() {
            let huge_addr = addr >> 9;
            *huge_pages.entry(huge_addr).or_insert(0) += 1;
        }

        term.clear_last_lines(1)?;
        term.write_line(&format!("Processed (decompressed): {}", HumanBytes(so_far)))?;
    }

    dump(&huge_pages);

    Ok(())
}

fn dump(hist: &HashMap<u64, u64, FxBuildHasher>) {
    let mut hist: Vec<_> = hist.iter().collect();
    hist.sort_unstable_by_key(|(_, v)| *v);
    hist.reverse();

    for (k, f) in hist.into_iter() {
        println!("{:16X}\t{:10}", k, f);
    }
}
