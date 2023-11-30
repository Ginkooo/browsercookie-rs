
# browsercookie-rs

A rust crate useful for extracting cookies from browsers. Inspired from [browsercookie](https://pypi.org/project/browsercookie/) python library.

## Library

### Usage

Using the library is quite simple

```rust
// Cargo.toml
[dependencies]
browsercookie-rs = { git="https://github.com/Ginkooo/browserscookie-rs.git", branch="main" }
```

```rust
use browsercookie::{CookieFinder, Browser, Attribute};

let mut cookie_jar = CookieFinder::builder()
    .with_regexp(Regex::new("google.com").unwrap(), Attribute::Domain)
    .with_browser(Browser::Firefox)
    .build
    .find()
    .await.unwrap();

let cookie = cookie_jar.get("some_cookie_name").unwrap();

println!("Cookie header string: Cookie: {}", cookie);
```

You can omit the `.with_` calls to get all cookies from all browsers.

A better example should be present in [browsercookies](src/bin.rs).

## Binary

The same crate should also give you a binary `browsercookies`, which should be usable from your favorite shell for crudely using frontend APIs for simple tooling.

```console
browsercookies --domain jira
```

## Install

```console
cargo install -f browsercookie-rs
```

## Feature Matrix

| TargetOS | Firefox | Chrome |
|----------|---------|--------|
| Linux    | ✔       | ✗      |
| macOS    | ✔       | ✗      |
| Windows  | ✗       | ✗      |

## Contributions

Contributions are very welcome. The easiest way to contribute is to look at the Python library [browser_cookie3](https://github.com/borisbabic/browser_cookie3), try to mimic the behavior that this library lacks, and submit a pull request. Make sure to format it, use Clippy, and include some tests.
