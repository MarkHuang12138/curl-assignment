use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::redirect::Policy;
use serde_json::Value;
use std::{env, fs::File, io::Write, str::FromStr};
use url::{ParseError, Url};

// print JSON with sorted keys
fn print_sorted_json(value: &Value) {
    if let Some(obj) = value.as_object() {
        let mut pairs: Vec<_> = obj.iter().collect();
        pairs.sort_by_key(|(k, _)| (*k).to_string());
        println!("{{");
        for (i, (k, v)) in pairs.iter().enumerate() {
            if i + 1 == pairs.len() {
                println!("  \"{}\": {}", k, v);
            } else {
                println!("  \"{}\": {},", k, v);
            }
        }
        println!("}}");
    } else {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    }
}

fn main() {
    //parameter Parsing
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <URL> [-X METHOD] [-d data] [--json JSON] [-H 'K: V']... [-I|--head] [-o FILE] [-L] [-s]");
        return;
    }

    let mut url_input: Option<String> = None;
    let mut method = "GET".to_string();
    let mut form_data = String::new(); // -d
    let mut json_raw: Option<String> = None; // --json
    let mut headers = HeaderMap::new(); // -H
    let mut head_only = false; // -I / --head
    let mut out_file: Option<String> = None; // -o
    let mut follow_redirects = false; // -L
    let mut silent = false; // -s

    let mut i = 1;
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
                    form_data = args[i + 1].clone();
                    i += 1;
                }
            }
            "--json" => {
                if i + 1 < args.len() {
                    json_raw = Some(args[i + 1].clone());
                    method = "POST".into();
                    i += 1;
                }
            }
            "-H" => {
                if i + 1 < args.len() {
                    let raw = &args[i + 1];
                    if let Some((k, v)) = raw.split_once(':') {
                        let name = HeaderName::from_str(k.trim()).expect("Invalid header name");
                        let value = HeaderValue::from_str(v.trim()).expect("Invalid header value");
                        headers.append(name, value);
                    } else {
                        eprintln!("Error: -H expects 'Name: Value'");
                        return;
                    }
                    i += 1;
                }
            }
            "-I" | "--head" => {
                head_only = true;
                method = "HEAD".into();
            }
            "-o" => {
                if i + 1 < args.len() {
                    out_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-L" => {
                follow_redirects = true;
            }
            "-s" => {
                silent = true;
            }

            token if !token.starts_with('-') && url_input.is_none() => {
                url_input = Some(token.to_string());
            }

            //unknown option exit
            x if x.starts_with('-') => {
                eprintln!("Unknown option: {}", x);
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let url_input = match url_input {
        Some(u) => u,
        None => {
            println!("Usage: cargo run -- <URL> [-X METHOD] [-d data] [--json JSON] [-H 'K: V']... [-I|--head] [-o FILE] [-L] [-s]");
            return;
        }
    };

    if !silent {
        println!("Requesting URL: {}", url_input);
        println!("Method: {}", method);
        if let Some(s) = &json_raw {
            println!("JSON: {}", s);
        } else if !form_data.is_empty() {
            println!("Data: {}", form_data);
        }
    }

    //URL Validation
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

    //client Build
    let client = Client::builder()
        .redirect(if follow_redirects {
            Policy::limited(10)
        } else {
            Policy::none()
        })
        .build()
        .expect("Failed to build client");

    //Content-Type
    if json_raw.is_some() && !headers.contains_key(CONTENT_TYPE) {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    } else if !form_data.is_empty() && !headers.contains_key(CONTENT_TYPE) {
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
    }

    //send Request
    let mut req = match method.as_str() {
        "GET" => client.get(parsed.clone()),
        "POST" => client.post(parsed.clone()),
        "HEAD" => client.head(parsed.clone()),
        _ => {
            eprintln!("Unsupported method: {}", method);
            return;
        }
    }
    .headers(headers);

    if let Some(s) = &json_raw {
        let v: Value = serde_json::from_str(s).unwrap_or_else(|e| panic!("Invalid JSON: {}", e));
        req = req.body(serde_json::to_string(&v).unwrap());
    } else if method == "POST" {
        req = req.body(form_data.clone());
    }

    let response = match req.send() {
        Ok(r) => r,
        Err(_) => {
            println!("Error: Unable to connect to the server. Perhaps the network is offline or the server hostname cannot be resolved.");
            return;
        }
    };

    // print only the response headers
    if head_only {
        for (k, v) in response.headers() {
            println!("{}: {}", k, v.to_str().unwrap_or("<binary>"));
        }
        return;
    }

    //non-2xx
    if !response.status().is_success() {
        println!(
            "Error: Request failed with status code: {}.",
            response.status().as_u16()
        );
        return;
    }

    //output file or saved file
    if let Some(path) = out_file {
        let mut file = File::create(&path).expect("cannot create file");
        let bytes = response.bytes().expect("read body failed");
        file.write_all(&bytes).expect("write body failed");
        if !silent {
            println!("Saved body to {}", path);
        }
    } else {
        let text = response.text().unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            if !silent {
                println!("Response body (JSON with sorted keys):");
            }
            print_sorted_json(&v);
        } else {
            if !silent {
                println!("Response body:");
            }
            println!("{}", text);
        }
    }
}
