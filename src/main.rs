use reqwest::blocking::get;
use std::env;
use url::{ParseError, Url};

fn main() {
    //read command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <URL>");
        return;
    }
    let url_str = &args[1];

    println!("Requesting URL: {}", url_str);
    println!("Method: GET");

    let parsed = Url::parse(url_str);
    let parsed = match parsed {
        Ok(u) => u,
        Err(e) => {
            use ParseError::*;
            let msg = match e {
                InvalidIpv6Address => "Error: The URL contains an invalid IPv6 address.",
                InvalidIpv4Address => "Error: The URL contains an invalid IPv4 address.",
                InvalidPort => "Error: The URL contains an invalid port number.",
                _ => "Error: The URL does not have a valid base protocol.",
            };
            println!("{}", msg);
            return;
        }
    };

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        println!("Error: The URL does not have a valid base protocol.");
        return;
    }

    let resp = match get(url_str) {
        Ok(r) => r,
        Err(e) => {
            println!("Request failed: {}", e);
            return;
        }
    };

    let body = match resp.text() {
        Ok(t) => t,
        Err(e) => {
            println!("Failed to read response body: {}", e);
            return;
        }
    };

    println!("Response body:\n{}", body);
}
