//! Demonstration of parsing LIST ACTIVE vs LIST NEWSGROUPS responses.
//!
//! This example shows that the parser can now handle both formats:
//! - LIST ACTIVE: name last first posting_status
//! - LIST NEWSGROUPS: name description...

use nntp_rs::response::Response;

fn main() {
    println!("=== LIST ACTIVE Format ===");
    let active_response = "215 List of newsgroups follows\r\n\
        comp.lang.c 12345 1 y\r\n\
        alt.binaries.pictures 9999 5000 n\r\n\
        misc.test 100 50 m\r\n\
        .\r\n";

    let parsed = Response::parse_str(active_response).unwrap();
    if let Response::NewsgroupList(groups) = parsed {
        println!("Parsed {} newsgroups:", groups.len());
        for group in groups {
            println!(
                "  {} (articles {}-{}, posting: {})",
                group.name, group.first, group.last, group.posting_status
            );
        }
    }

    println!("\n=== LIST NEWSGROUPS Format ===");
    let newsgroups_response = "215 Descriptions in form \"group description\"\r\n\
        comp.lang.c Discussion about C programming\r\n\
        alt.binaries.pictures Pictures in binary format\r\n\
        misc.test A test newsgroup for testing purposes\r\n\
        .\r\n";

    let parsed = Response::parse_str(newsgroups_response).unwrap();
    if let Response::NewsgroupList(groups) = parsed {
        println!("Parsed {} newsgroups:", groups.len());
        for group in groups {
            println!(
                "  {} (default values: articles {}-{}, posting: {})",
                group.name, group.first, group.last, group.posting_status
            );
        }
    }

    println!("\n=== Mixed Format (shouldn't happen but handles gracefully) ===");
    let mixed_response = "215 List follows\r\n\
        comp.lang.c 12345 1 y\r\n\
        alt.binaries.pictures Pictures only\r\n\
        misc.test 100 50 m\r\n\
        .\r\n";

    let parsed = Response::parse_str(mixed_response).unwrap();
    if let Response::NewsgroupList(groups) = parsed {
        println!("Parsed {} newsgroups:", groups.len());
        for group in groups {
            println!(
                "  {} (articles {}-{}, posting: {})",
                group.name, group.first, group.last, group.posting_status
            );
        }
    }
}
