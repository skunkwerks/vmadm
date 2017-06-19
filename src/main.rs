//! vmadm compatible jail manager

#![deny(trivial_numeric_casts,
        missing_docs,
        unstable_features,
        unused_import_braces,
)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate clap;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate uuid;
extern crate toml;

#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
#[macro_use]
extern crate slog_scope;
use slog::Drain;


use std::result;
use std::error::Error;
use std::io;

mod zfs;

pub mod jdb;
use jdb::JDB;

mod config;
use config::Config;

pub mod errors;
use errors::{GenericError, NotFoundError};

/// Custom Drain logic
struct RuntimeLevelFilter<D> {
    drain: D,
    level: u64,
}

/// Drain to define log leve via `-v` flags
impl<D> Drain for RuntimeLevelFilter<D>
where
    D: Drain,
{
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> result::Result<Self::Ok, Self::Err> {
        let current_level = match self.level {
            1 => slog::Level::Critical,
            2 => slog::Level::Error,
            3 => slog::Level::Warning,
            4 => slog::Level::Info,
            5 => slog::Level::Debug,
            6 => slog::Level::Trace,
            _ => return Ok(None),
        };
        if record.level().is_at_least(current_level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}


/// Main function
fn main() {
    let exit_code = run();
    std::process::exit(exit_code)
}

fn run() -> i32 {
    use clap::App;
    let yaml = load_yaml!("cli.yml");
    let mut help_app = App::from_yaml(yaml).version(crate_version!());
    let matches = App::from_yaml(yaml).version(crate_version!()).get_matches();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let level = matches.occurrences_of("verbose");
    let drain = RuntimeLevelFilter {
        drain: drain,
        level: level,
    }.fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let root = slog::Logger::root(drain, o!());

    let _guard = slog_scope::set_global_logger(root);

    let config: Config = Config::new().unwrap();
    let r = if matches.is_present("startup") {
        match matches.subcommand() {
            ("", None) => startup(&config),
            _ => Err(GenericError::bx("Can not use startup with a subcommand")),
        }
    } else {
        match matches.subcommand() {
            ("list", Some(list_matches)) => list(&config, list_matches),
            ("create", Some(create_matches)) => create(&config, create_matches),
            ("update", Some(update_matches)) => update(&config, update_matches),
            ("destroy", Some(destroy_matches)) => destroy(&config, destroy_matches),
            ("start", Some(start_matches)) => start(&config, start_matches),
            ("stop", Some(stop_matches)) => stop(&config, stop_matches),
            ("", None) => {
                help_app.print_help().unwrap();
                Ok(0)
            }
            _ => unreachable!(),
        }
    };
    crit!("Execution done");
    match r {
        Ok(0) => 0,
        Ok(exit_code) => exit_code,
        Err(e) => {
            crit!("error: {}", e);
            1
        }
    }
}

fn startup(_conf: &Config) -> Result<i32, Box<Error>> {
    Ok(0)
}

fn start(_conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    Ok(0)
}

fn stop(_conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    Ok(0)
}

fn update(_conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    Ok(0)
}

fn list(conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    db.print()
}

fn create(conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let mut db = JDB::open(conf)?;
    let jail: jdb::JailConfig = serde_json::from_reader(io::stdin())?;
    let mut dataset = conf.settings.pool.clone();
    dataset.push('/');
    dataset.push_str(jail.image_uuid.clone().as_str());
    let entry = db.insert(jail)?;
    let snap = zfs::snapshot(dataset.as_str(), entry.uuid.as_str())?;
    zfs::clone(snap.as_str(), entry.root.as_str())?;
    Ok(0)
}

fn destroy(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let mut db = JDB::open(conf)?;
    let uuid = value_t!(matches, "uuid", String).unwrap();
    debug!("Destroying jail {}", uuid);
    match db.get(uuid.as_str()) {
        Some(entry) => {
            let origin = zfs::origin(entry.root.as_str());
            match zfs::destroy(entry.root.as_str()) {
                Ok(_) => debug!("zfs dataset deleted: {}", entry.root),
                Err(e) => warn!("failed to delete dataset: {}", e),
            };
            match origin {
                Ok(origin) => {
                    zfs::destroy(origin.as_str())?;
                    debug!("zfs snapshot deleted: {}", origin)
                }
                Err(e) => warn!("failed to delete origin: {}", e),
            }
        }
        None => return Err(NotFoundError::bx("Could not find VM")),
    };
    db.remove(uuid.as_str())?;
    Ok(0)
}