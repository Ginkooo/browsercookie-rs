//! # browsercookie-rs
//!
//! Browsercookie-rs crate allows you to gather cookies from browsers
//! on the system and return them in a CookieJar, so that it can be
//! used with other http libraries like Hyper etc..
//!
//! ```rust,ignore
//! use Browsercookie::{Browser, Browsercookies};
//!
//! let mut bc = Browsercookies::new();
//! let domain_regex = Regex::new(".*");
//! bc.from_browser(Browser::Firefox, &domain_regex).expect("Failed to get firefox browser cookies");
//! if let Ok(cookie_header) = bc.to_header(&domain_regex) as Result<String, Box<Error>> {
//!     println!("Cookies extracted");
//! }
//! ```
//!
//! Using above `to_header` returns a string to be used with http clients as a header
//! directly.
//!
//! ```rust,ignore
//! use reqwest::header;
//! use Browsercookie::{Browser, Browsercookies};
//!
//! let mut bc = Browsercookies::new();
//! let domain_regex = Regex::new("www.rust-lang.org");
//! bc.from_browser(Browser::Firefox, &domain_regex).expect("Failed to get firefox browser cookies");
//!
//! if let Ok(cookie_header) = bc.to_header(&domain_regex) as Result<String, Box<Error>> {
//!     let mut headers = header::HeaderMap::new();
//!     headers.insert(header::COOKIE, header::HeaderValue::from_str(&cookie_header));
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


mod firefox;
pub mod errors;

/// All supported browsers
pub enum Browser {
    Firefox
}

/// Main struct facilitating operations like collection & parsing of cookies from browsers
pub struct Browsercookies {
    pub cj: Box<CookieJar>
}

impl Browsercookies {
    pub fn new() -> Browsercookies {
        Browsercookies {
            cj: Box::new(CookieJar::new())
        }
    }

    pub fn from_browser(&mut self, b: Browser, domain_regex: &Regex) -> Result<(), Box<dyn Error>> {
        match b {
            Browser::Firefox => return firefox::load(&mut self.cj, domain_regex)
        }
    }

    pub fn to_header(&self, domain_regex: &Regex) -> Result<String, Box<dyn Error>> {
        let mut header = String::from("");
        for cookie in self.cj.iter() {
            if domain_regex.is_match(cookie.domain().unwrap()) {
                header.push_str(&format!("{}={}; ", cookie.name(), cookie.value()));
            }
        }
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firefox() {
        let mut bc = Browsercookies::new();
        let domain_regex = Regex::new(".*").unwrap();
        bc.from_browser(Browser::Firefox, &domain_regex).expect("Failed to get firefox browser cookies");
        if let Ok(cookie_header) = bc.to_header(&domain_regex) as Result<String, Box<dyn Error>> {
            println!("{}", cookie_header);
        }
    }
}
