use byteorder::{LittleEndian, ReadBytesExt};
use cookie::{Cookie, CookieJar};
#[allow(unused_imports)]
use dirs::home_dir;
use futures::TryStreamExt;
use ini::Ini;
use lz4::block::decompress;
use memmap::MmapOptions;
use regex::Regex;
use serde_json::Value;
use sqlx::prelude::*;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqliteConnection;
use std::error::Error;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::errors::BrowsercookieError;
use crate::Attribute;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct MozCookie {
    host: String,
    name: String,
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
        path.push(".mozilla/firefox/profiles.ini");
    } else if cfg!(target_os = "windows") {
        path.push("AppData\\Roaming\\Mozilla\\Firefox\\profiles.ini");
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
            .section(sec)
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

async fn load_from_sqlite(
    sqlite_path: &Path,
    cookie_jar: &mut CookieJar,
    domain_regex: &(Regex, Attribute),
) -> Result<(), Box<dyn Error>> {
    let options = SqliteConnectOptions::new()
        .filename(sqlite_path)
        .read_only(true)
        .immutable(true);
    let mut conn = SqliteConnection::connect_with(&options)
        .await
        .expect("Could not connect to cookies.sqlite");
    let mut query = sqlx::query("SELECT name, value, host from moz_cookies").fetch(&mut conn);

    while let Some(row) = query.try_next().await? {
        let name: String = row.get(0);
        let value: String = row.get(1);
        let host: String = row.get(2);

        if domain_regex.0.is_match(&host) {
            cookie_jar.add(
                Cookie::build((name, value))
                    .domain(host)
                    .path("/")
                    .secure(false)
                    .http_only(false)
                    .build(),
            );
        }
    }
    Ok(())
}

async fn load_from_recovery(
    recovery_path: &Path,
    cookie_jar: &mut CookieJar,
    regex_and_attribute: &(Regex, Attribute),
) -> Result<(), Box<dyn Error>> {
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
            if regex_and_attribute.0.is_match(&cookie.host) {
                cookie_jar.add(
                    Cookie::build((cookie.name, cookie.value))
                        .domain(cookie.host)
                        .path(cookie.path)
                        .secure(cookie.secure)
                        .http_only(cookie.httponly)
                        .build(),
                );
            }
        }
    }
    Ok(())
}

pub(crate) async fn load(
    cookie_jar: &mut CookieJar,
    regex_and_attribute: &(Regex, Attribute),
) -> Result<(), Box<dyn Error>> {
    // Returns a CookieJar if following steps go right
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

    let mut recovery_path = profile_path.clone();
    recovery_path.push("sessionstore-backups/recovery.jsonlz4");

    if recovery_path.exists() {
        load_from_recovery(&recovery_path, cookie_jar, regex_and_attribute).await?;
    }

    let mut sqlite_path = profile_path.clone();

    if sqlite_path.exists() {
        sqlite_path.push("cookies.sqlite");
        load_from_sqlite(&sqlite_path, cookie_jar, regex_and_attribute).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recovery_load() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/recovery.jsonlz4");
        let mut bcj = Box::new(CookieJar::new());

        let domain_re = Regex::new(".*").unwrap();
        load_from_recovery(&path, &mut bcj, &(domain_re, Attribute::Domain))
            .await
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

    #[tokio::test]
    async fn test_sqlite_load() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/Profiles/1qbuu7ux.default/cookies.sqlite");
        let domain_re = Regex::new(".*").unwrap();
        let mut bcj = Box::new(CookieJar::new());
        load_from_sqlite(&path, &mut bcj, &(domain_re, Attribute::Domain))
            .await
            .unwrap();

        let cookie = bcj.get("somename").unwrap();

        assert_eq!(cookie.value(), "somevalue");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.domain(), Some("somehost"));
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
