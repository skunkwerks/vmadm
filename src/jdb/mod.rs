use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::str;
use uuid::Uuid;


use serde_json;
use serde_derive;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "new_uuid")]
    pub uuid: String,
    alias: String,
    ram: u64,
    cpu: u64,
    disk: u64,
}
#[derive(Debug, Serialize, Deserialize)]
struct IdxEntry {
    version: u32,
    uuid: String,
    root: String,
    state: String,
    jail_type: String,
}

fn new_uuid() -> String {
    Uuid::new_v4().hyphenated().to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct Index {
    pub version: u32,
    pub entries: Vec<IdxEntry>,
}

#[derive(Debug)]
pub struct JDB<'a> {
    dir: &'a Path,
    index: Index,
}

impl<'a> JDB<'a> {
    pub fn open(path: &'a Path) -> Result<Self, Box<Error>> {
        match File::open(path) {
            Ok(file) => {
                let index: Index = serde_json::from_reader(file)?;
                Ok(JDB {
                    index: index,

                    dir: path.parent().unwrap(),
                })
            }
            Err(_) => {
                let mut entries: Vec<IdxEntry> = Vec::new();
                let index: Index = Index {
                    version: 0,
                    entries: entries,
                };
                let db = JDB {
                    index: index,
                    dir: path.parent().unwrap(),
                };
                db.save();
                Ok(db)

            }

        }
    }
    pub fn insert(self: &'a mut JDB<'a>, config: Config) -> Result<Config, Box<Error>> {
        let mut path = self.dir.join(config.uuid.clone());
        path.set_extension("json");
        let file = File::create(&path)?;
        let mut root = String::from("/jails/");
        root.push_str(&config.uuid.clone());
        let mut e = IdxEntry{
            version: 0,
            uuid: config.uuid.clone(),
            state: String::from("installing"),
            jail_type: String::from("base"),
            root: root,
        };
        self.index.entries.push(e);
        self.save();
        serde_json::to_writer(file, &config)?;
        Ok(config)
    }

    fn config(self: &'a JDB<'a>, entry: &IdxEntry) -> Result<Config, Box<Error>> {
        let mut config_path = self.dir.join(entry.uuid.clone());
        config_path.set_extension("json");
        let config_file = File::open(config_path)?;
        let mut conf: Config = serde_json::from_reader(config_file)?;
        Ok(conf)
    }

    fn save(self: &'a JDB<'a>) -> Result<usize, Box<Error>> {
        let mut path = self.dir.join("index");
        let mut file = File::create(path)?;
        serde_json::to_writer(file, &self.index);
        Ok(self.index.entries.len())
    }
    pub fn find(self: &'a JDB<'a>, uuid: String) -> Option<&'a IdxEntry> {
        for e in &self.index.entries {
            if e.uuid == uuid {
                return Some(e);
            }
        }
        None
    }
    pub fn print(self: &'a JDB<'a>) {
        println!(
            "{:37} {:5} {:8} {:17} {}",
            "UUID",
            "TYPE",
            "RAM",
            "STATE",
            "ALIAS"
        );
        for e in &(self.index.entries) {
            self.print_entry(e);
        }
    }

    fn print_entry(self: &'a JDB<'a>, entry: &IdxEntry) {
        let conf = self.config(entry).unwrap();
        println!(
            "{:37} {:5} {:8} {:17} {}",
            conf.uuid,
            "OS",
            conf.ram,
            entry.state,
            conf.alias
        )
    }
}