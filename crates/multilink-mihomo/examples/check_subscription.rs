//! Read a URL/payload from stdin. Only aggregate diagnostics go to stdout.
use std::io::{self, Read};
fn main() {
    let mut input = String::new();
    io::stdin()
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut input)
        .expect("Read stdin");
    let result = if input.trim().starts_with("https://") {
        multilink_mihomo::fetch_subscription(input.trim())
            .and_then(|body| multilink_core::import_subscription(&body))
    } else {
        multilink_core::import_subscription(&input)
    };
    match result {
        Ok(report) => {
            println!(
                "Compatible nodes: {}; skipped: {}",
                report.nodes.len(),
                report.skipped.len()
            );
            let mut reasons = std::collections::BTreeMap::new();
            for error in report.skipped {
                let reason = error
                    .split_once(": ")
                    .map(|(_, reason)| reason.to_owned())
                    .unwrap_or(error);
                *reasons.entry(reason).or_insert(0) += 1;
            }
            for (reason, count) in reasons {
                println!("{count}: {reason}");
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
