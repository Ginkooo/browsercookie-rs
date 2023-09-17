//! # browsercookie-rs
//!
//! Browsercookie-rs crate allows you to gather cookies from browsers
//! on the system and return them in a CookieJar, so that it can be
//! used with other http libraries like Hyper etc..
//!
//! ```rust,ignore
//! use Browsercookie::{Browser, Attribute, CookieFinder};
//!
//! let mut cookie_jar = CookieFinder::builder()
//!     .with_regexp(Regex::new(".*").unwrap(), Attribute::Domain)
//!     .with_browser(Browser::Firefox)
//!     .build().find().await.unwrap();
//!
//! println!("{}", cookie_jar.get("searched_cookie_name").unwrap());
//!
//! ```
//!
//! Using above `to_header` returns a string to be used with http clients as a header
//! directlytrue.
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
use cookie::CookieJar;
use regex::Regex;
use std::collections::HashSet;

#[macro_use]
extern crate serde;

pub mod errors;
mod firefox;

/// All supported browsers
#[derive(PartialEq, Eq, Hash)]
pub enum Browser {
    Firefox,
}

pub enum Attribute {
    Name,
    Value,
    Domain,
    Path,
}

#[derive(Default)]
pub struct CookieFinder {
    regex_and_attribute_pairs: Vec<(Regex, Attribute)>,
    browsers: HashSet<Browser>,
}

#[derive(Default)]
pub struct CookieFinderBuilder {
    cookie_finder: CookieFinder,
}

impl CookieFinderBuilder {
    pub fn with_regexp(mut self, regex: Regex, attribute: Attribute) -> Self {
        self.cookie_finder
            .regex_and_attribute_pairs
            .push((regex, attribute));
        self
    }

    pub fn with_browser(mut self, browser: Browser) -> Self {
        self.cookie_finder.browsers.insert(browser);
        self
    }

    pub fn build(self) -> CookieFinder {
        self.cookie_finder
    }
}

impl CookieFinder {
    pub fn builder() -> CookieFinderBuilder {
        CookieFinderBuilder::default()
    }

    pub async fn find(&self) -> CookieJar {
        let mut cookie_jar = CookieJar::new();
        for regex_and_attribute in &self.regex_and_attribute_pairs {
            for browser in &self.browsers {
                match browser {
                    Browser::Firefox => {
                        firefox::load(&mut cookie_jar, regex_and_attribute)
                            .await
                            .expect("Something went wrong loading the cookies from Firefox");
                    }
                }
            }
        }
        cookie_jar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_firefox() {
        let domain_regex = Regex::new(".*").unwrap();
        let cookies = CookieFinder::builder()
            .with_regexp(domain_regex, Attribute::Domain)
            .with_browser(Browser::Firefox)
            .build()
            .find()
            .await;
        assert_eq!(cookies.iter().count(), 2);
        let recovery_cookie = cookies.get("name").unwrap();
        assert_eq!(recovery_cookie.value(), "value");
        assert_eq!(recovery_cookie.domain(), Some("httpbin.org"));
        assert_eq!(recovery_cookie.path(), Some("/"));

        let sqlite_cookie = cookies.get("somename").unwrap();

        assert_eq!(sqlite_cookie.value(), "somevalue");
        assert_eq!(sqlite_cookie.path(), Some("/"));
        assert_eq!(sqlite_cookie.domain(), Some("somehost"));
    }
}
