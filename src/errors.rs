//! Exported errors if library users wish to handle certain failure cases
use std::fmt;
use std::error;

#[derive(Debug)]
pub enum BrowsercookieError {
    ProfileMissing(String),
    InvalidProfile(String),
    InvalidCookieStore(String),
    InvalidRecovery(String)
}

impl fmt::Display for BrowsercookieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error in fetching browsercookies")
    }
}

// This is important for other errors to wrap this one.
impl error::Error for BrowsercookieError {
    fn description(&self) -> &str {
        "Error in fetching browsercookies"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}
