
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
extern crate aud;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate toml;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate rand;
extern crate reqwest;
extern crate chrono;
extern crate tempfile;
extern crate bzip2;
extern crate flate2;

//extern crate indicatif;

#[macro_use]
extern crate prettytable;

extern crate uuid;
use uuid::Uuid;

#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
#[macro_use]
extern crate slog_scope;
extern crate slog_bunyan;
use slog::Drain;

use std::result;
use std::error::Error;
use std::io;
use std::fs::OpenOptions;
use std::fs::File;

use aud::{Failure, Adventure, Saga};

mod brand;
use brand::Brand;
mod zfs;
mod images;
mod jails;
use jails::Jail;

mod jail_config;
mod update;

use jail_config::JailConfig;

mod jdb;
use jdb::{JDB, IdxEntry};

mod config;
use config::Config;

mod errors;
use errors::{GenericError, ValidationErrors};

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
            0 => return Ok(None),
            1 => slog::Level::Critical,
            2 => slog::Level::Error,
            3 => slog::Level::Warning,
            4 => slog::Level::Info,
            5 => slog::Level::Debug,
            _ => slog::Level::Trace,
        };
        if record.level().is_at_least(current_level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}
/// Main function
#[cfg(target_os = "freebsd")]
fn main() {
    let exit_code = run();
    std::process::exit(exit_code)
}

#[cfg(not(target_os = "freebsd"))]
fn main() {
    println!("Jails are not supported, running in dummy mode");
    let exit_code = run();
    std::process::exit(exit_code)
}

fn run() -> i32 {
    use clap::App;
    let yaml = load_yaml!("cli.yml");
    let mut help_app = App::from_yaml(yaml).version(crate_version!());
    let matches = App::from_yaml(yaml).version(crate_version!()).get_matches();

    /// console logger
    let decorator = slog_term::TermDecorator::new().build();
    let term_drain = slog_term::FullFormat::new(decorator).build().fuse();
    let level = matches.occurrences_of("verbose");
    let term_drain = RuntimeLevelFilter {
        drain: term_drain,
        level: level,
    }.fuse();
    let term_drain = slog_async::Async::new(term_drain).build().fuse();

    /// fiel logger
    let log_path = "/var/log/vmadm.log";
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(log_path)
        .unwrap();

    // create logger
    let file_drain = slog_bunyan::default(file).map(slog::Fuse);
    let file_drain = slog_async::Async::new(file_drain).build().fuse();

    let drain = slog::Duplicate::new(file_drain, term_drain).fuse();

    let root = slog::Logger::root(
        drain,
        o!("req_id" => Uuid::new_v4().hyphenated().to_string()),
    );

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
            ("delete", Some(delete_matches)) => delete(&config, delete_matches),
            ("start", Some(start_matches)) => start(&config, start_matches),
            ("reboot", Some(reboot_matches)) => reboot(&config, reboot_matches),
            ("stop", Some(stop_matches)) => stop(&config, stop_matches),
            ("get", Some(get_matches)) => get(&config, get_matches),
            ("info", Some(info_matches)) => info(&config, info_matches),
            ("console", Some(console_matches)) => console(&config, console_matches),
            ("images", Some(image_matches)) => images(&config, image_matches),
            ("config", Some(config_matches)) => hv_config(&config, config_matches),

            ("", None) => {
                help_app.print_help().unwrap();
                println!();
                Ok(0)
            }
            _ => unreachable!(),
        }
    };
    debug!("Execution done");
    match r {
        Ok(0) => 0,
        Ok(exit_code) => exit_code,
        Err(e) => {
            println!("{}", e);
            crit!("error: {}", e);
            1
        }
    }
}

fn startup(conf: &Config) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    for e in db.iter() {
        let jail = db.get(&e.uuid)?;
        if jail.config.autoboot && jail.outer.is_none() {
            println!("Starting jail {}", jail.idx.uuid);
            jail.start(conf)?;
        } else {
            println!("Skipping startup for jail {}", jail.idx.uuid);

        }
    };
    Ok(0)
}

fn start(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("Starting jail {}", uuid.hyphenated());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(Jail { outer: Some(_), .. }) => {
            println!("The vm is alredy started");
            Err(GenericError::bx("VM is already started"))
        }
        Ok(jail) => {
            println!("Starting jail {}", jail.idx.uuid);
            jail.start(conf)
        }
    }
}

fn reboot(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("deleteing jail {}", uuid.hyphenated());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(Jail { outer: None, .. }) => {
            println!("The vm is not running");
            Err(GenericError::bx("The vm is not running"))
        }
        Ok(jail) => {
            println!("Rebooting jail {}", uuid);
            jail.stop(conf)?;
            jail.start(conf)
        }
    }
}

fn get(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid = value_t!(matches, "uuid", Uuid).unwrap();
    debug!("Starting jail {}", uuid.hyphenated().to_string());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(Jail { config: conf, .. }) => {
            let j = serde_json::to_string_pretty(&conf)?;
            println!("{}", j);
            Ok(0)
        }
    }
}

fn info(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("Getting jail info {}", uuid.hyphenated());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(_jail) => {
            println!("Unable to get info for jail.\n");
            Ok(0)
        }
    }
}

#[derive(Serialize)]
struct Info {
    networks: Vec<String>
}

fn hv_config(conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let info = Info{
        networks: conf.settings.networks.keys().map(|v| v.clone()).collect::<Vec<String>>()
    };
    debug!("Getting hypervisor info.");
    let j = serde_json::to_string_pretty(&info)?;
    println!("{}", j);
    Ok(0)
}

fn console(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("Starting jail {}", uuid.hyphenated());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(Jail { inner: None, .. }) => {
            println!("The vm is not running");
            Err(GenericError::bx("VM is not running"))
        }
        Ok(jail) => {
            let brand = jail.brand(conf)?;
            let mut child = brand.login.spawn(&jail, conf)
                .expect("failed to execute jexec");
            let ecode = child.wait().expect("failed to wait on child");
            if ecode.success() {
                Ok(0)
            } else {
                Err(GenericError::bx("Failed to execute jail console"))
            }
        }
    }
}

fn stop(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("stopping jail {}", uuid.hyphenated());
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(Jail { outer: None, .. }) => {
            println!("The vm is alredy stopped");
            Err(GenericError::bx("VM is already stooped"))
        }
        Ok(jail) => {
            println!("Stopping jail {}", uuid);
            jail.stop(conf)
        }
    }
}

fn list(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    db.print(
        matches.is_present("headerless"),
        matches.is_present("parsable"),
    )
}

fn update(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    let update = match value_t!(matches, "file", String) {
        Err(_) => {
            debug!("Reading from STDIN");
            update::JailUpdate::from_reader(io::stdin())?
        }
        Ok(file) => {
            debug!("Reading from file"; "file" => file.clone() );
            update::JailUpdate::from_reader(File::open(file)?)?
        }
    };
    match db.get(&uuid) {
        Err(e) => Err(e),
        Ok(jail) => {
            let c = update.apply(jail.config, jail.idx)?;
            // TODO: This is ugly ...
            let mut db = JDB::open(conf)?;
            db.update(c)
        }
    }
}

fn create(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let jail = match value_t!(matches, "file", String) {
        Err(_) => {
            debug!("Reading from STDIN");
            jail_config::JailConfig::from_reader(io::stdin())?
        }
        Ok(file) => {
            debug!("Reading from file"; "file" => file.clone() );
            jail_config::JailConfig::from_reader(File::open(file)?)?
        }
    };
    let mut dataset = conf.settings.pool.clone();
    dataset.push('/');
    dataset.push_str(jail.image_uuid.hyphenated().to_string().as_str());

    #[derive(Debug, Clone)]
    struct CreateState<'a> {
        conf: &'a Config,
        uuid: Uuid,
        dataset: String,
        config: JailConfig,
        entry: Option<IdxEntry>,
        snapshot: Option<String>,
        root: Option<String>,
    }

    let state = CreateState {
        conf,
        uuid: jail.uuid.clone(),
        dataset,
        config: jail.clone(),
        entry: None,
        snapshot: None,
        root: None,
    };
    fn insert_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        match JDB::open(state.conf) {
            Ok(mut db) => {
                match db.insert(state.config.clone()) {
                    Ok(entry) => Ok(CreateState {
                        conf: state.conf,
                        uuid: state.uuid,
                        dataset: state.dataset,
                        config: state.config,
                        entry: Some(entry),
                        snapshot: state.snapshot,
                        root: state.root,
                    }),
                    Err(error) => Err(Failure::new(state, error)),
                }
            }
            Err(error) => Err(Failure::new(state, error)),
        }
    };
    fn insert_down(state: CreateState) -> CreateState {
        crit!("Rolling back insert");
        match JDB::open(state.conf) {
            Ok(mut db) => {
                let _ = db.remove(&state.uuid);
            }
            Err(_error) => (),
        };
        state
    };

    fn snap_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        match zfs::snapshot(
            state.dataset.as_str(),
            state.uuid.hyphenated().to_string().as_str(),
        ) {
            Ok(snap) => Ok(CreateState {
                conf: state.conf,
                uuid: state.uuid,
                dataset: state.dataset,
                config: state.config,
                entry: state.entry,
                snapshot: Some(snap),
                root: state.root,
            }),
            Err(error) => Err(Failure::new(state, error)),
        }
    }
    fn snap_down(state: CreateState) -> CreateState {
        crit!("Rolling back snapshot");
        match state.snapshot.clone() {
            Some(snap) => {
                let _ = zfs::destroy(snap.as_str());
                state
            }
            None => state,
        }
    }

    fn clone_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        match state.snapshot.clone() {
            Some(snap) => {
                match state.entry.clone() {
                    Some(entry) => {
                        match zfs::clone(snap.as_str(), entry.root.as_str()) {
                            Ok(_) => Ok(CreateState {
                                conf: state.conf,
                                uuid: state.uuid,
                                dataset: state.dataset,
                                config: state.config,
                                entry: state.entry,
                                snapshot: state.snapshot,
                                root: Some(entry.root),
                            }),
                            Err(error) => Err(Failure::new(state, error)),
                        }
                    }
                    None => Err(Failure::new(state, GenericError::bx("No root to clone"))),
                }
            }
            None => Err(Failure::new(state, GenericError::bx("No snap to clone"))),
        }
    }
    fn clone_down(state: CreateState) -> CreateState {
        crit!("Rolling back clone");
        match state.root.clone() {
            Some(root) => {
                let _ = zfs::destroy(root.as_str());
                state
            }
            None => state,
        }
    }
    fn quota_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        let s1 = state.clone();
        let entry = s1.entry.unwrap();
        let root = entry.root.as_str();
        let quota = s1.config.quota;
        match zfs::quota(root, quota) {
            Ok(_) => Ok(state),
            Err(e) => Err(Failure::new(state, e)),
        }

    }
    fn quota_down(state: CreateState) -> CreateState {
        crit!("Rolling back init");
        state
    }
    fn init_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        let s1 = state.clone();
        let jail = Jail{
            idx: & s1.entry.unwrap(),
            config: s1.config,
            inner: None,
            outer: None,
        };
        let state1 = CreateState{
            conf: state.conf,
            uuid: state.uuid,
            dataset: state.dataset,
            config: state.config,
            entry: state.entry,
            snapshot: state.snapshot,
            root: state.root,
         };
        match jail.init(state1.conf) {
            Ok(_) => Ok(state1),
            Err(err) => Err(Failure::new(state1, err))
        }
    }
    fn init_down(state: CreateState) -> CreateState {
        crit!("Rolling back init");
        state
    }
    fn brand_install_up(state: CreateState) -> Result<CreateState, Failure<CreateState>> {
        let s1 = state.clone();
        let jail = Jail{
            idx: & s1.entry.unwrap().clone(),
            config: s1.config,
            inner: None,
            outer: None,
        };
        match Brand::load(state.config.brand.as_str(), state.conf) {
            Err(_) => Err(Failure::new(state, GenericError::bx("invalid brand"))),
            Ok(brand)  => {
                match brand.install.output(&jail, state.conf) {
                    Ok(_) => Ok(state),
                    Err(_) => Err(Failure::new(state, GenericError::bx("failed to initilize brand")))
                }

            }
        }
    }
    fn brand_install_down(state: CreateState) -> CreateState {
        crit!("Rolling back clone");
        state
    }
    let saga = Saga::new(vec![
        Adventure::new(insert_up, insert_down),
        Adventure::new(snap_up, snap_down),
        Adventure::new(clone_up, clone_down),
        Adventure::new(quota_up, quota_down),
        Adventure::new(init_up, init_down),
        Adventure::new(brand_install_up, brand_install_down),
    ]);
    match jail.errors(conf) {
        Some(errors) => Err(ValidationErrors::bx(errors)),
        None => 
            match saga.tell(state) {
                Ok(state) => {
                    println!("Created jail {}", state.uuid);
                    Ok(0)
                }
                Err(failure) => Err(failure.to_error()),
            }
    }

}

fn delete(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let mut db = JDB::open(conf)?;
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    debug!("deleteing jail {}", uuid.hyphenated());
    let res = match db.get(&uuid) {
        Ok(jail) => {
            if jail.outer.is_some() {
                println!("Stopping jail {}", uuid);
                jail.stop(conf)?;
            };
            let origin = zfs::origin(jail.idx.root.as_str());
            match zfs::destroy(jail.idx.root.as_str()) {
                Ok(_) => debug!("zfs dataset deleted: {}", jail.idx.root),
                Err(e) => warn!("failed to delete dataset: {}", e),
            };
            match origin {
                Ok(origin) => {
                    zfs::destroy(origin.as_str())?;
                    debug!("zfs snapshot deleted: {}", origin)
                }
                Err(e) => warn!("failed to delete origin: {}", e),
            };
            println!("deleted jail {}", uuid);
            Ok(0)
        }
        Err(e) => Err(e),
    };
    db.remove(&uuid)?;
    res
}

fn images(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
        match matches.subcommand() {
            ("avail", Some(avail_matches)) => images_avail(&conf, avail_matches),
            ("list", Some(list_matches)) => images_list(&conf, list_matches),
            ("get", Some(get_matches)) => images_get(&conf, get_matches),
            ("show", Some(show_matches)) => images_show(&conf, show_matches),
            ("import", Some(import_matches)) => images_import(&conf, import_matches),
            ("", None) => {
                Ok(0)
            }
            _ => unreachable!(),
        }
}

fn images_avail(conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    images::avail(conf)
}

fn images_list(conf: &Config, _matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    images::list(conf)
}

fn images_get(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    images::get(conf, uuid)
}

fn images_show(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    images::show(conf, uuid)
}

fn images_import(conf: &Config, matches: &clap::ArgMatches) -> Result<i32, Box<Error>> {
    let uuid_string = value_t!(matches, "uuid", String).unwrap();
    let uuid = Uuid::parse_str(uuid_string.as_str()).unwrap();
    images::import(conf, uuid)
}
