use byteorder::{LittleEndian, ReadBytesExt};
use cookie::{Cookie, CookieJar};
#[allow(unused_imports)]
use dirs::home_dir;
use ini::Ini;
use lz4::block::decompress;
use memmap::MmapOptions;
use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::errors::BrowsercookieError;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct MozCookie {
    host: String,
    name: String,
    #[allow(dead_code)]
    originAttributes: Value,
    path: String,
    value: String,

    #[serde(default)]
    secure: bool,

    #[serde(default)]
    httponly: bool,
}

#[cfg(test)]
fn get_master_profile_path() -> PathBuf {
    // Only used for tests, should do this a better way by mocking
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/resources/profiles.ini");
    path
}

#[cfg(not(test))]
fn get_master_profile_path() -> PathBuf {
    let mut path = home_dir().expect("Unable to find home directory");
    if cfg!(target_os = "macos") {
        path.push("Library/Application Support/Firefox/profiles.ini");
    } else if cfg!(target_os = "linux") {
        path.push(".mozilla/firefox/profiles.ini")
    }
    path
}

fn get_default_profile_path(master_profile: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let profiles_conf: Ini;
    let mut default_profile_path = PathBuf::from(master_profile);
    default_profile_path.pop();

    match Ini::load_from_file(master_profile) {
        Err(_) => {
            return Err(Box::new(BrowsercookieError::InvalidProfile(String::from(
                "Unable to parse firefox ini profile",
            ))))
        }
        Ok(p) => profiles_conf = p,
    }

    let default_install_profile_directory = &profiles_conf
        .iter()
        .find(|&section| section.0.as_ref().is_some_and(|s| s.starts_with("Install")))
        .and_then(|section| {
            section
                .1
                .iter()
                .find(|&(key, _)| key == "Default")
                .map(|s| s.1)
        });

    if default_install_profile_directory.is_some() {
        default_profile_path.push(default_install_profile_directory.unwrap());
        return Ok(default_profile_path);
    }

    for (sec, _) in &profiles_conf {
        let section = profiles_conf
            .section(sec.clone())
            .ok_or("Invalid profile section")?;
        match section.get("Default").and(section.get("Path")) {
            Some(path) => {
                default_profile_path.push(path);
                break;
            }
            None => continue,
        }
    }
    Ok(default_profile_path)
}

fn load_from_recovery(
    recovery_path: &Path,
    bcj: &mut Box<CookieJar>,
    domain_regex: &Regex,
) -> Result<bool, Box<dyn Error>> {
    let recovery_file = File::open(recovery_path)?;
    let recovery_mmap = unsafe { MmapOptions::new().map(&recovery_file)? };

    if recovery_mmap.len() <= 8
        || recovery_mmap.get(0..8).ok_or("Invalid recovery")? != "mozLz40\0".as_bytes()
    {
        return Err(Box::new(BrowsercookieError::InvalidRecovery(String::from(
            "Firefox invalid recovery archive",
        ))));
    }

    let mut rdr = Cursor::new(recovery_mmap.get(8..12).ok_or("Invalid recovery")?);
    let uncompressed_size = rdr.read_i32::<LittleEndian>().ok();

    let recovery_json_bytes = decompress(
        recovery_mmap.get(12..).ok_or("Invalid recovery")?,
        uncompressed_size,
    )?;

    let recovery_json: Value = serde_json::from_slice(&recovery_json_bytes)?;
    for c in recovery_json["cookies"]
        .as_array()
        .ok_or("Invalid recovery")?
    {
        if let Ok(cookie) =
            serde_json::from_value(c.clone()) as Result<MozCookie, serde_json::error::Error>
        {
            // println!("Loading for {}: {}={}", cookie.host, cookie.name, cookie.value);
            if domain_regex.is_match(&cookie.host) {
                bcj.add(
                    Cookie::build(cookie.name, cookie.value)
                        .domain(cookie.host)
                        .path(cookie.path)
                        .secure(cookie.secure)
                        .http_only(cookie.httponly)
                        .finish(),
                );
            }
        }
    }
    Ok(true)
}

fn load_from_sqlite(
    sqlite_path: &Path,
    bcj: &mut Box<CookieJar>,
    domain_regex: &Regex,
) -> Result<bool, Box<dyn Error>> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY;

    let conn = Connection::open_with_flags(sqlite_path, flags)?;

    let mut query =
        conn.prepare("SELECT name, value, host, path, isSecure, isHttpOnly FROM moz_cookies")?;

    let cookies = query.query_map([], |row| {
        Ok(
            Cookie::build(row.get::<_, String>(0)?, row.get::<_, String>(1)?)
                .domain(row.get::<_, String>(2)?)
                .path(row.get::<_, String>(3)?)
                .secure(row.get(4)?)
                .http_only(row.get(5)?)
                .finish(),
        )
    })?;

    cookies
        .filter_map(|c| c.ok())
        .filter(|c| {
            domain_regex.is_match(
                c.domain()
                    .expect("We set the domain above, so it should always exist"),
            )
        })
        .for_each(|c| bcj.add(c));
    Ok(true)
}

fn load_cookies_from_all_sources(
    profile_path: PathBuf,
    bcj: &mut Box<CookieJar>,
    domain_regex: &Regex,
) -> Result<(), Box<dyn Error>> {
    let mut recovery_path = profile_path.clone();
    let mut sqlite_path = profile_path;
    recovery_path.push("sessionstore-backups/recovery.jsonlz4");
    sqlite_path.push("cookies.sqlite");

    let sqlite_load_result = load_from_sqlite(&sqlite_path, bcj, domain_regex).ok();
    let recovery_load_result = load_from_recovery(&recovery_path, bcj, domain_regex).ok();

    if recovery_load_result.is_none() && sqlite_load_result.is_none() {
        return Err(Box::new(BrowsercookieError::InvalidCookieStore(
            String::from("Could not load cookies from Firefox sqlite cookie store nor from recovery file"),
        )));
    }

    Ok(())
}

pub(crate) fn load(bcj: &mut Box<CookieJar>, domain_regex: &Regex) -> Result<(), Box<dyn Error>> {
    // Returns a CookieJar on heap if following steps go right
    //
    // 1. Get default profile path for firefox from master ini profiles config.
    // 2. Load cookies from recovery json (sessionstore-backups/recovery.jsonlz4)
    //    of the default profile.
    let master_profile_path = get_master_profile_path();
    if !master_profile_path.exists() {
        return Err(Box::new(BrowsercookieError::ProfileMissing(String::from(
            "Firefox profile path doesn't exist",
        ))));
    }

    let profile_path = get_default_profile_path(&master_profile_path)?;

    load_cookies_from_all_sources(profile_path, bcj, domain_regex)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_load() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/recovery.jsonlz4");
        let mut bcj = Box::new(CookieJar::new());

        let domain_re = Regex::new(".*").unwrap();
        load_from_recovery(&path, &mut bcj, &domain_re)
            .expect("Failed to load from firefox recovery json");

        let c = bcj
            .get("taarId")
            .expect("Failed to get cookie from firefox recovery");

        assert_eq!(c.value(), "value");
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.secure(), Some(true));
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.domain(), Some("addons.mozilla.org"));
    }

    #[test]
    fn test_sqlite_load() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/cookies.sqlite");
        let mut bcj = Box::new(CookieJar::new());

        let domain_re = Regex::new(".*").unwrap();
        load_from_sqlite(&path, &mut bcj, &domain_re)
            .expect("Failtd to load cookies from firefox sqlite");

        let c = bcj
            .get("some_name")
            .expect("Failed to get cookie from firefox sqlite");
        assert_eq!(c.value(), "some_value");
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.secure(), Some(true));
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.domain(), Some("some_host"));
    }

    #[test]
    fn will_both_recovery_file_and_sqlite_file_do_not_exist() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/Profiles/profile_with_no_cookie_files");
        let mut bcj = Box::new(CookieJar::new());

        let domain_re = Regex::new(".*").unwrap();
        let result = load_cookies_from_all_sources(path, &mut bcj, &domain_re);

        assert!(result.is_err_and(|e| e.is::<BrowsercookieError>()));
    }

    #[test]
    fn test_master_profile() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/profiles.ini");

        let default_profile_path =
            get_default_profile_path(&path).expect("Failed to parse master firefox profile");

        assert!(default_profile_path.ends_with(PathBuf::from("Profiles/1qbuu7ux.default")));
    }

    #[test]
    fn test_master_profile_with_install_section() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/profiles_with_install.ini");

        let default_profile_path =
            get_default_profile_path(&path).expect("Failed to parse master firefox profile");

        assert!(default_profile_path.ends_with(PathBuf::from("Profiles/dmjvfd1o.default-release")));
    }
}
