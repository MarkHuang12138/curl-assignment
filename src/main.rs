use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use std::env;
use url::{ParseError, Url};

fn main() {
    //retrieve command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <URL> [-X POST] [-d data]");
        return;
    }

    let mut url_input = String::new();
    let mut method = "GET".to_string();
    let mut data = String::new(); // POST

    //URL
    url_input = args[1].trim().to_string();

    //iterate over the remaining parameters
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-X" => {
                if i + 1 < args.len() {
                    method = args[i + 1].to_uppercase();
                    i += 1;
                }
            }
            "-d" => {
                if i + 1 < args.len() {
                    data = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Requesting URL: {}", url_input);
    println!("Method: {}", method);
    if !data.is_empty() {
        println!("Data: {}", data);
    }

    //URL Check
    let parsed = match Url::parse(&url_input) {
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
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        println!("Error: The URL does not have a valid base protocol.");
        return;
    }

    let client = Client::new();
    let response = if method == "POST" {
        client
            .post(&url_input)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(data.clone())
            .send()
    } else {
        client.get(&url_input).send()
    };

    let response = match response {
        Ok(r) => r,
        Err(_) => {
            println!("Error: Unable to connect to the server. Perhaps the network is offline or the server hostname is invalid.");
            return;
        }
    };

    if !response.status().is_success() {
        println!(
            "Error: Request failed with status code: {}.",
            response.status().as_u16()
        );
        return;
    }

    let text = response.text().unwrap_or_default();

    //attempt to parse as JSON
    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        println!("Response body (JSON with sorted keys):");
        if let Some(obj) = v.as_object() {
            let mut sorted = obj.clone().into_iter().collect::<Vec<_>>();
            sorted.sort_by_key(|(k, _)| k.clone());
            println!("{{");
            for (k, val) in sorted {
                println!("  \"{}\": {},", k, val);
            }
            println!("}}");
        } else {
            //if it is an array, print
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        }
    } else {
        //not JSON, output the original text
        println!("Response body:\n{}", text);
    }
}
