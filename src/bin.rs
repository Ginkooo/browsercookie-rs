use regex::Regex;
use clap::{Arg, App};
use browsercookie::{Browser, Browsercookies};

#[macro_use]
extern crate clap;

fn curl_output(bc: &Browsercookies, domain_regex: &Regex) {
    print!("Cookie: {}", bc.to_header(domain_regex).unwrap());
}

fn python_output(bc: &Browsercookies, domain_regex: &Regex) {
    print!("{{'Cookie': '{}'}}", bc.to_header(domain_regex).unwrap());
}

fn main() {
    let matches = App::new("browsercookies")
                        .version(crate_version!())
                        .author(crate_authors!())
                        .about(crate_description!())
                        .arg(Arg::with_name("domain")
                             .short("d")
                             .long("domain")
                             .value_name("DOMAIN_REGEX")
                             .required(true)
                             .help("Sets a domain filter for cookies")
                             .takes_value(true))
                        .arg(Arg::with_name("browser")
                             .short("b")
                             .long("browser")
                             .value_name("BROWSER")
                             .multiple(true)
                             .default_value("firefox")
                             .help("Accepted values: firefox (only one can be provided)")
                             .takes_value(true))
                        .arg(Arg::with_name("output")
                             .short("o")
                             .long("output")
                             .value_name("OUTPUT_FORMAT")
                             .help("Accepted values: curl,python (only one can be provided)")
                             .default_value("curl")
                             .takes_value(true))
                        .get_matches();

    let mut bc = Browsercookies::new();
    let domain_regex = Regex::new(matches.value_of("domain").unwrap()).unwrap();

    for b in  matches.values_of("browser").unwrap() {
        if b == "firefox"  {
            bc.from_browser(Browser::Firefox, &domain_regex).expect("Failed to get cookies from firefox");
        }
    }

    match matches.value_of("output").unwrap() {
        "curl" => curl_output(&bc, &domain_regex),
        "python" => python_output(&bc, &domain_regex),
        _ => ()
    }
}
