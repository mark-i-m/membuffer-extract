use std::io::{BufRead, Read};

use flate2::read::ZlibDecoder;

const HEADER_U64S: usize = 3;
const HEADER_SIZE: usize = std::mem::size_of::<u64>() * HEADER_U64S;

/// A raw trace.
pub struct Trace<R: BufRead> {
    /// The raw compressed trace.
    decoder: ZlibDecoder<R>,

    /// State for the suspended decoder so far...
    state: TraceIterState,

    /// The common prefix for this chunk.
    common: u64,
    /// The number of common bytes (the length of `common` in bytes).
    prefix_len: usize,

    /// The data we are currently processing.
    data: Vec<u8>,
    /// The index of the next byte in `data` to be processed.
    i: usize,
}

enum TraceIterState {
    /// We need to read the next chunk.
    NeedHead,

    /// We are working our way through this chunk.
    HaveHead,

    /// We are done with all data.
    Done,
}

impl<R: BufRead> Trace<R> {
    pub fn new(input: R) -> Self {
        let decoder = ZlibDecoder::new(input);

        Self {
            decoder,
            state: TraceIterState::NeedHead,
            common: 0,
            prefix_len: 0,
            data: vec![],
            i: 0,
        }
    }

    pub fn so_far(&self) -> u64 {
        self.decoder.total_out()
    }

    /// If needed, read another chunk's metadata and get ready to process it.
    fn buffer_if_needed(&mut self) {
        if let TraceIterState::NeedHead = self.state {
            let mut head = [0u8; HEADER_SIZE];
            if let Err(e) = self.decoder.read_exact(&mut head) {
                log::debug!("Error while reading head: {:?}", e);
                self.state = TraceIterState::Done;
                return;
            }

            let (common, prefix_len, n): (u64, u64, u64) = match unsafe {
                std::slice::from_raw_parts_mut(head.as_mut_ptr() as *mut u64, HEADER_U64S)
            } {
                &mut [common, prefix_len, n] => (common, prefix_len, n),
                _ => unreachable!(),
            };

            if n == 0 {
                self.state = TraceIterState::Done;
                return;
            }

            let mut data: Vec<u8> = vec![0; ((8 - prefix_len) * n) as usize];
            if let Err(e) = self.decoder.read_exact(&mut data) {
                log::debug!("Error while reading data: {:?}", e);
                self.state = TraceIterState::Done;
                return;
            }

            // Compute the new internal state.
            self.state = TraceIterState::HaveHead;
            self.common = common;
            self.prefix_len = prefix_len as usize;
            self.data = data;
            self.i = 0;
        }
    }

    /// Read and return the next portion of the trace. This will return an emtpy array iff we reach
    /// the end of the trace or their is an error.
    fn read_chunk(&mut self) -> Vec<u64> {
        // Read the next chunk if needed.
        self.buffer_if_needed();

        match self.state {
            TraceIterState::Done => return vec![],
            TraceIterState::NeedHead => panic!("NeedHead after reading next chunk!"),
            TraceIterState::HaveHead => {} // ok
        }

        let result = self
            .data
            .as_slice()
            .chunks_exact((8 - self.prefix_len) as usize)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0, |acc: u64, (i, b)| acc | (*b as u64) << (i as usize * 8))
            })
            .map(|uniq| uniq | self.common)
            .collect();

        self.state = TraceIterState::NeedHead;

        result
    }
}

impl<R: BufRead> Iterator for Trace<R> {
    type Item = (Vec<u64>, u64);

    /// Returns the next address in the trace.
    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.read_chunk();

        if chunk.is_empty() {
            return None;
        }

        Some((chunk, self.so_far()))
    }
}
