use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;

use console::Term;

use fxhash::FxBuildHasher;

use indicatif::HumanBytes;

use structopt::StructOpt;

use trace::Trace;

pub mod trace;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "membuffer-extract",
    about = "Extracts compressed membuffer output."
)]
pub struct Config {
    #[structopt(parse(from_os_str))]
    pub file: PathBuf,
}

pub fn process(config: &Config, cap: usize) -> std::io::Result<()> {
    let file = std::fs::File::open(&config.file)?;
    let trace = Trace::new(BufReader::with_capacity(cap, file));

    let mut huge_pages: HashMap<_, _, FxBuildHasher> = HashMap::default();

    let term = Term::stdout();
    term.write_line(&format!(
        "Processed (decompressed): {}",
        HumanBytes(trace.so_far())
    ))?;

    trace.for_each(|(chunk, so_far)| {
        for addr in chunk.into_iter() {
            let huge_addr = addr >> 9;
            //huge_pages.upsert(huge_addr, || 1, |v| *v += 1);
            *huge_pages.entry(huge_addr).or_insert(0) += 1;
        }

        term.clear_last_lines(1).unwrap();
        term.write_line(&format!("Processed (decompressed): {}", HumanBytes(so_far)))
            .unwrap();
    });

    dump(huge_pages);

    Ok(())
}

fn dump(hist: HashMap<u64, u64, FxBuildHasher>) {
    let mut hist: Vec<_> = hist.into_iter().collect();
    hist.sort_unstable_by_key(|(_, v)| *v);
    hist.reverse();

    for (k, f) in hist.into_iter() {
        println!("{:16X}\t{:10}", k, f);
    }
}
