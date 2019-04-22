//! # browsercookie-rs
//!
//! Browsercookie-rs crate allows you to gather cookies from browsers
//! on the system and return them in a CookieJar, so that it can be
//! used with other http libraries like Hyper etc..
//!
//! ```rust
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
//! Using above cookiejar with hyper is quite simple
//!
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
