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
    /// The trace file.
    pub file: PathBuf,

    #[structopt(long)]
    /// The period to print intermediate stats, if any.
    pub period: Option<usize>,

    #[structopt(long)]
    /// Filter to the top N pages.
    pub top: Option<usize>,
}

struct AddrInfo {
    first_access: usize,
    last_access: usize,
    num_accesses: usize,
}

pub struct Collector {
    num_addrs: usize,
    huge_pages: HashMap<u64, AddrInfo, FxBuildHasher>,

    period: Option<usize>,
    filter: Option<usize>,
}

impl AddrInfo {
    pub fn new(first_access: usize) -> Self {
        Self {
            first_access,
            last_access: first_access,
            num_accesses: 0,
        }
    }
}

impl Collector {
    pub fn new(period: Option<usize>, filter: Option<usize>) -> Self {
        Self {
            num_addrs: 0,
            huge_pages: HashMap::default(),
            period,
            filter,
        }
    }

    pub fn collect(&mut self, addr: u64) {
        let huge_addr = addr >> 9;

        let entry = self
            .huge_pages
            .entry(huge_addr)
            .or_insert(AddrInfo::new(self.num_addrs));

        entry.num_accesses += 1;
        entry.last_access = self.num_addrs;
        self.num_addrs += 1;

        if let Some(period) = self.period {
            if self.num_addrs % period == 0 {
                self.dump();
            }
        }
    }

    fn dump(&self) {
        let mut hist: Vec<_> = self.huge_pages.iter().collect();
        hist.sort_unstable_by_key(|(_, v)| v.num_accesses);
        hist.reverse();

        let last = if let Some(filter) = self.filter {
            filter
        } else {
            hist.len()
        };

        for (k, f) in hist.drain(..last) {
            println!(
                "{:16X}\t{:10}\t{:10}",
                k,
                f.num_accesses,
                f.last_access - f.first_access
            );
        }

        println!("===");
    }
}

pub fn process(config: &Config, cap: usize) -> std::io::Result<()> {
    let file = std::fs::File::open(&config.file)?;
    let trace = Trace::new(BufReader::with_capacity(cap, file));

    let term = Term::stderr();
    term.write_line(&format!(
        "Processed (decompressed): {}",
        HumanBytes(trace.so_far())
    ))?;

    let mut collector = Collector::new(config.period, config.top);

    trace.for_each(|(chunk, so_far)| {
        chunk.into_iter().for_each(|addr| collector.collect(addr));

        term.clear_last_lines(1).unwrap();
        term.write_line(&format!("Processed (decompressed): {}", HumanBytes(so_far)))
            .unwrap();
    });

    collector.dump();

    Ok(())
}
