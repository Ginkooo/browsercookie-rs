use regex::Regex;
use browsercookie::{Browser, Browsercookies};

fn main() {
    let mut browsercookies = Browsercookies::new();
    let jira_regex = Regex::new("google.com").unwrap();
    browsercookies.from_browser(Browser::Firefox, &jira_regex).expect("Failed to get cookies from firefox");
    println!("{}", browsercookies.to_header(&jira_regex).expect("Invalid regex"));
}
