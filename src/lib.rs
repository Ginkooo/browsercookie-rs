use cookie::CookieJar;
use std::error::Error;

#[macro_use] extern crate serde;

mod firefox;
pub mod errors;

pub enum Browser {
    Firefox
}

pub fn load(b: Browser) -> Result<Box<CookieJar>, Box<Error>> {
    match b {
        Browser::Firefox => return firefox::load()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
