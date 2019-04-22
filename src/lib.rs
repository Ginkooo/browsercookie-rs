//! # browsercookie-rs
//!
//! Browsercookie-rs crate allows you to gather cookies from browsers
//! on the system and return them in a CookieJar, so that it can be
//! used with other http libraries like Hyper etc..
//!
//! ```rust,ignore
//! use Browsercookie::get_browsercookies;
//! use Browsercookie::Browser;
//!
//! let res = get_browsercookies(Browser::Firefox);
//! if let Ok(cj) = res {
//!     println!("Cookies extracted");
//!     // .... Do something with cookiejar ...
//! }
//! ```
//!
//! Using above cookiejar with `get_cookieheader` returns a string to
//! be used with http clients as a header directly.
//!
//! ```rust,ignore
//! use reqwest::header;
//! use Browsercookie::*;
//!
//! let res = get_browsercookies(Browser::Firefox);
//! if let Ok(cj) = res {
//!     let mut headers = header::HeaderMap::new();
//!     headers.insert(header::COOKIE, header::HeaderValue::from_str(
//!         get_cookieheader(cj, "www.rust-lang.org"));
//!
//!     let client = reqwest::Client::builder()
//!         .default_headers(headers)
//!         .build()?;
//!     let res = client.get("https://www.rust-lang.org").send()?;
//! }
//! ```
use regex::Regex;
use cookie::CookieJar;
use std::error::Error;

#[macro_use] extern crate serde;

mod firefox;
pub mod errors;

pub enum Browser {
    Firefox
}

pub fn get_browsercookies(b: Browser) -> Result<Box<CookieJar>, Box<Error>> {
    match b {
        Browser::Firefox => return firefox::load()
    }
}

pub fn get_cookieheader(bcj: Box<CookieJar>, domain: &str) -> Result<String, Box<Error>> {
    let mut header = String::from("");
    let domain_re = Regex::new(domain)?;
    for cookie in bcj.iter() {
        if domain_re.is_match(domain) {
            header.push_str(&format!("{}={}; ", cookie.name(), cookie.value()));
        }
    }
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firefox() {
        let bcj = get_browsercookies(Browser::Firefox).expect("Failed to get firefox browser cookies");
        if let Ok(cookie_header) = get_cookieheader(bcj, ".*") as Result<String, Box<Error>> {
            assert_eq!(cookie_header, "name=value; ");
        }
    }
}
