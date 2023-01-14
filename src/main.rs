use error_chain::error_chain;
use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::StatusCode;
use std::fs::File;
use std::str::FromStr;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        Header(reqwest::header::ToStrError);
    }
}

#[derive(Debug)]
struct PartialRangeIterator {
    start: u64,
    end: u64,
    step: u64,
}

impl PartialRangeIterator {
    fn new(start: u64, end: u64, buffer_size: u64) -> Result<Self> {
        if buffer_size == 0 {
            panic!("step must be greater than 0");
        }
        Ok(PartialRangeIterator {
            start,
            end,
            step: buffer_size,
        })
    }
}

// implement Iterator for PartialRangeIterator
impl Iterator for PartialRangeIterator {
    type Item = Result<(u64, u64)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }
        let end = std::cmp::min(self.start + self.step, self.end);
        let range = (self.start, end);
        self.start = end;
        Some(Ok(range))
    }
}

fn main() -> Result<()> {
    let url = "https://images.unsplash.com/photo-1442512595331-e89e73853f31?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1770&q=80";
    const CHUNK_SIZE: u64 = 10240;

    // create a client
    let client = reqwest::blocking::Client::new();
    let response = client.head(url).send().unwrap();
    let content_length = response
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or("response has not content-length header")?;
    let content_length =
        u64::from_str(content_length.to_str()?).map_err(|_| "content-length is not a valid u64")?;
    let mut output_file = File::create("test.bin").unwrap();
    println!("starting download...");
    for range in PartialRangeIterator::new(0, content_length - 1, CHUNK_SIZE) {
        println!("downloading range: {:?}", range);

        // send a request with range
        let range_string = format!("bytes={}-{}", range.start, range.end);
        let range_header = HeaderValue::from_str(&range_string).unwrap();
        let request = client.get(url).header(RANGE, range_header);
        // send request
        let mut response = request.send()?;

        // check status code
        let status = response.status();
        if status != StatusCode::PARTIAL_CONTENT {
            panic!("unexpected status code: {}", status);
        }
        // write to file
        std::io::copy(&mut response, &mut output_file)?;
    }
    let content = response.text()?;
    std::io::copy(&mut content.as_bytes(), &mut output_file)?;

    println!("File downloaded successfully!");
    Ok(())
}
