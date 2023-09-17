use browsercookie::{Attribute, Browser, CookieFinder};
use clap::{App, Arg};
use regex::Regex;

#[macro_use]
extern crate clap;

async fn curl_output(cookie_finder: &CookieFinder) {
    let cookie_jar = cookie_finder.find().await;
    let cookie = cookie_jar.iter().last().expect("Cookie not found");
    print!("Cookie: {}", cookie);
}

async fn python_output(cookie_finder: &CookieFinder) {
    let cookie_jar = cookie_finder.find().await;
    let cookie = cookie_jar.iter().last().expect("Cookie not found");
    print!("{{'Cookie': '{}'}}", cookie);
}

#[tokio::main]
async fn main() {
    let matches = App::new("browsercookies")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .value_name("DOMAIN_REGEX")
                .required(true)
                .help("Sets a domain filter for cookies")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("browser")
                .short("b")
                .long("browser")
                .value_name("BROWSER")
                .multiple(true)
                .default_value("firefox")
                .help("Accepted values: firefox (only one can be provided)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .conflicts_with("output")
                .value_name("COOKIE_NAME")
                .help("Specify a cookie name to output only that value")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT_FORMAT")
                .help("Accepted values: curl,python (only one can be provided)")
                .default_value("curl")
                .takes_value(true),
        )
        .get_matches();

    let domain_regex = Regex::new(matches.value_of("domain").unwrap()).unwrap();

    let mut builder = CookieFinder::builder().with_regexp(domain_regex, Attribute::Domain);

    for b in matches.values_of("browser").unwrap() {
        if b == "firefox" {
            builder = builder.with_browser(Browser::Firefox);
        }
    }

    if let Some(cookie_name) = matches.value_of("name") {
        builder.build().find().await.iter().for_each(|c| {
            if c.name() == cookie_name {
                println!("{}", c.value());
            }
        });
    } else {
        match matches.value_of("output").unwrap() {
            "curl" => curl_output(&builder.build()).await,
            "python" => python_output(&builder.build()).await,
            _ => (),
        }
    }
}
