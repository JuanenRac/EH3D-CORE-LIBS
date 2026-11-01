//! Mutation fuzzing without dependencies: damaged variants of a valid file.
//!
//! The same file is copied into the tests of the three reader crates.

#![allow(dead_code)]
// Every slice below starts and ends on a character boundary taken from `char_indices` or on a digit run.
#![allow(clippy::string_slice)]

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const INSERTS: [&str; 3] = ["é", "€", "😀"];
const BIG_NUMBERS: [&str; 3] = ["99999999999999999999999999", "9999999999", "1e999"];

/// Every damaged variant of `sample`, none identical to a plain copy of it.
pub fn mutants(sample: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bounds: Vec<usize> = sample
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(sample.len()))
        .collect();

    for &at in &bounds {
        let (head, tail) = sample.split_at(at);
        // non-ASCII text at every character boundary
        for insert in INSERTS {
            out.push(format!("{head}{insert}{tail}"));
        }
        // the file cut at every character
        out.push(head.to_string());
    }
    // one character removed
    for pair in bounds.windows(2) {
        let (head, tail) = (&sample[..pair[0]], &sample[pair[1]..]);
        out.push(format!("{head}{tail}"));
    }
    // one byte removed or cut in the middle of a character (invalid UTF-8 becomes U+FFFD)
    let bytes = sample.as_bytes();
    for i in 0..bytes.len() {
        let mut cut = bytes.to_vec();
        cut.remove(i);
        out.push(String::from_utf8_lossy(&cut).into_owned());
        out.push(String::from_utf8_lossy(&bytes[..i]).into_owned());
    }
    // one line repeated
    let lines: Vec<&str> = sample.split_inclusive('\n').collect();
    for i in 0..lines.len() {
        let mut copy = lines.clone();
        copy.insert(i, lines[i]);
        out.push(copy.concat());
    }
    // extra digits and huge numbers in every run of digits
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut end = i;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            out.push(format!(
                "{}{}{}",
                &sample[..end],
                "9".repeat(40),
                &sample[end..]
            ));
            for big in BIG_NUMBERS {
                out.push(format!("{}{}{}", &sample[..i], big, &sample[end..]));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Runs `reader` on every variant of every sample; fails on a panic or when it takes too long.
pub fn no_variant_breaks_the_reader<F>(samples: &[&str], reader: F)
where
    F: Fn(&str) + Send + 'static,
{
    let owned: Vec<String> = samples.iter().map(|s| (*s).to_string()).collect();
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut count = 0usize;
        for sample in &owned {
            for variant in mutants(sample) {
                let started = std::time::Instant::now();
                let attempt =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader(&variant)));
                if attempt.is_err() {
                    let _ = tx.send(Err(format!("panic on {variant:?}")));
                    return;
                }
                if started.elapsed() > Duration::from_secs(2) {
                    let _ = tx.send(Err(format!("too slow on {variant:?}")));
                    return;
                }
                count += 1;
            }
        }
        let _ = tx.send(Ok(count));
    });
    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Ok(count)) => assert!(count > 500, "only {count} variants were tried"),
        Ok(Err(message)) => panic!("{message}"),
        Err(_) => panic!("the reader did not finish in time (endless loop?)"),
    }
    let _ = worker.join();
}
