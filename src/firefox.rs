use ini::Ini;
use std::fs::File;
use dirs::home_dir;
use std::io::Cursor;
use std::error::Error;
use cookie::{Cookie, CookieJar};
use memmap::MmapOptions;
use lz4::block::decompress;
use std::path::{Path, PathBuf};
use serde_json::{Value};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::errors::BrowsercookieError;

#[derive(Deserialize, Debug)]
struct MozCookie {
    host: String,
    name: String,
    originAttributes: Value,
    path: String,
    value: String,

    #[serde(default)]
    secure: bool,

    #[serde(default)]
    httponly: bool
}

fn get_master_profile_path() -> PathBuf {
    let mut path: PathBuf = home_dir().expect("Unable to find home directory");
    if cfg!(target_os = "macos") {
        path.push("Library/Application Support/Firefox/profiles.ini");
    } else if cfg!(target_os = "linux") {
        path.push(".mozilla/firefox/profiles.ini")
    }
    path
}

fn get_default_profile_path(master_profile: &Path) -> Result<PathBuf, Box<Error>> {
    let profiles_conf: Ini;
    let mut default_profile_path = PathBuf::from(master_profile);
    default_profile_path.pop();

    match Ini::load_from_file(&master_profile) {
        Err(_) => return Err(Box::new(BrowsercookieError::InvalidProfile(String::from("Unable to parse firefox ini profile")))),
        Ok(p) => profiles_conf = p
    }

    for (sec, _) in &profiles_conf {
        let section = profiles_conf.section(sec.clone()).ok_or("Invalid profile section")?;
        match section.get("Default").and(section.get("Path")) {
            Some(path) => {
                default_profile_path.push(path);
                break
            },
            _ => println!("Not default profile")
        }
    }
    Ok(default_profile_path)
}

fn load_from_recovery(recovery_path: &Path, bcj: &mut Box<CookieJar>) -> Result<bool, Box<Error>> {
    let recovery_file = File::open(recovery_path)?;
    let recovery_mmap = unsafe { MmapOptions::new().map(&recovery_file)? };

    if recovery_mmap.len() <= 8 || recovery_mmap.get(0..8).ok_or("Invalid recovery")? != "mozLz40\0".as_bytes() {
        return Err(Box::new(BrowsercookieError::InvalidRecovery(String::from("Firefox invalid recovery archive"))))
    }
    // println!("{:?}", recovery_mmap.get(0..8)?);

    let mut rdr = Cursor::new(recovery_mmap.get(8..12).ok_or("Invalid recovery")?);
    let uncompressed_size = rdr.read_i32::<LittleEndian>().ok();

    let recovery_json_bytes = decompress(recovery_mmap.get(12..).ok_or("Invalid recovery")?, uncompressed_size)?;

    let recovery_json: Value = serde_json::from_slice(&recovery_json_bytes)?;
    for c in recovery_json["cookies"].as_array().ok_or("Invalid recovery")? {
        if let Ok(cookie) = serde_json::from_value(c.clone()) as Result<MozCookie, serde_json::error::Error> {
            bcj.add_original(Cookie::build(cookie.name, cookie.value)
                             .domain(cookie.host)
                             .path(cookie.path)
                             .secure(cookie.secure)
                             .http_only(cookie.httponly)
                             .finish());
        }
    }
    Ok(true)
}

pub(crate) fn load() -> Result<Box<CookieJar>, Box<Error>>  {
    let mut bcj = Box::new(CookieJar::new());

    let master_profile_path = get_master_profile_path();
    if !master_profile_path.exists() {
        return Err(Box::new(BrowsercookieError::ProfileMissing(String::from("Firefox profile path doesn't exist"))))
    }

    let profile_path = get_default_profile_path(&master_profile_path)?;

    let mut recovery_path = profile_path;
    recovery_path.push("sessionstore-backups/recovery.jsonlz4");

    if !recovery_path.exists() {
        return Err(Box::new(BrowsercookieError::InvalidCookieStore(String::from("Firefox invalid cookie store"))))
    }

    load_from_recovery(&recovery_path, &mut bcj)?;

    println!("{:?}", bcj);
    Ok(bcj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        load();
    }
}
