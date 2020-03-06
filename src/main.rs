use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use flate2::bufread::ZlibDecoder;

use fxhash::FxBuildHasher;

use structopt::StructOpt;

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
    let file = BufReader::new(file);
    let mut decoder = ZlibDecoder::new(file);

    let mut huge_pages: HashMap<_, _, FxBuildHasher> = HashMap::default();

    loop {
        //println!("Consumed bytes: {}", decoder.total_out());

        let mut head = vec![0u8; 8 * 3];
        if let Err(..) = decoder.read_exact(&mut head) {
            break;
        }

        let (common, prefix_len, n): (u64, u64, u64) =
            match unsafe { std::slice::from_raw_parts_mut(head.as_mut_ptr() as *mut u64, 3) } {
                &mut [common, prefix_len, n] => (common, prefix_len, n),
                _ => unreachable!(),
            };

        //println!("common: {:X}, prefix_len: {}, n: {}", common, prefix_len, n);

        let mut data: Vec<u8> = vec![0; ((8 - prefix_len) * n) as usize];
        decoder.read_exact(&mut data)?;

        for val in data
            .as_slice()
            .chunks_exact((8 - prefix_len) as usize)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0, |acc: u64, (i, b)| acc | (*b as u64) << (i as usize * 8))
            })
        {
            let addr = val | common;
            let huge_addr = addr >> 9;

            *huge_pages.entry(huge_addr).or_insert(0) += 1;
            //println!("{:x}", val | common);
        }

        //dump(&huge_pages);
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
