use std;
use std::io::Read;
use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Output};

use toml;

use jails::Jail;
use config::Config;

#[derive(Debug, Deserialize)]
pub struct Step {
    pub cmd: String,
    pub args: Vec<String>
}
fn expand(fmt: String, jail: &Jail, conf: &Config) -> String {
    let inner_id = match jail.inner {
        Some(e) => e.id,
        _ => 0
    };
    let outer_id = match jail.outer {
        Some(e) => e.id,
        _ => 0
    };

    let mut brand_root = PathBuf::from(conf.settings.brand_dir.as_str());
    brand_root.push(jail.config.brand.as_str());
    let mut jail_root = String::new();
    jail_root.push('/');
    jail_root.push_str(jail.idx.root.as_str());
    fmt.clone()
        .replace("{inner_id}", inner_id.to_string().as_str())
        .replace("{ounter_id}", outer_id.to_string().as_str())
        .replace("{jail_uuid}", jail.idx.uuid.hyphenated().to_string().as_str())
        .replace("{jail_root}", jail_root.as_str())
        .replace("{brand_root}", brand_root.to_string_lossy().as_ref())
        .replace("{hostname}", jail.config.hostname.as_str())
}

impl Step {
    fn cmd(&self, jail: &Jail, conf: &Config) -> String{
        expand(self.cmd.clone(), jail, conf)
    }
    fn args(&self, jail: &Jail, conf: &Config) -> Vec<String>{
        self.args.clone().into_iter().map(
            |arg| expand(arg, jail, conf)
        ).collect()
    }
    #[cfg(not(target_os = "freebsd"))]
    pub fn output(&self, jail: &Jail, conf: &Config) -> Result<Output, std::io::Error> {
        let command = self.cmd(jail, conf);
        let args = self.args(jail, conf);
        debug!("[BRAND] Running command";
               "command" => command.clone(),
               "args" => args.clone().join(" "),
               "scope" => "brand",
               "brand" => jail.config.brand.as_str());
        Command::new("echo").args(args).output()
    }
    #[cfg(target_os = "freebsd")]
    pub fn output(&self, jail: &Jail, conf: &Config) -> Result<Output, std::io::Error> {
        let command = self.cmd(jail, conf);
        let args = self.args(jail, conf);
        debug!("[BRAND] Running command";
               "command" => command.clone(),
               "args" => args.clone().join(" "),
               "scope" => "brand",
               "brand" => jail.config.brand.as_str());
        Command::new(command).args(args).output()
    }

    #[cfg(not(target_os = "freebsd"))]
    pub fn spawn(&self, jail: &Jail, conf: &Config) -> Result<std::process::Child, std::io::Error> {
        let command = self.cmd(jail, conf);
        let args = self.args(jail, conf);
        debug!("[BRAND] Running command";
               "command" => command.clone(),
               "args" => args.clone().join(" "),
               "scope" => "brand",
               "brand" => jail.config.brand.as_str());
        Command::new("echo").args(args).spawn()
    }
    #[cfg(target_os = "freebsd")]
    pub fn spawn(&self, jail: &Jail, conf: &Config) -> Result<std::process::Child, std::io::Error> {
        let command = self.cmd(jail, conf);
        let args = self.args(jail, conf);
        debug!("[BRAND] Running command";
               "command" => command.clone(),
               "args" => args.clone().join(" "),
               "scope" => "brand",
               "brand" => jail.config.brand.as_str());
        Command::new(command).args(args).spawn()
    }
    pub fn to_string(&self, jail: &Jail, conf: &Config) -> String {
        let mut cmd = self.cmd(jail, conf);
        cmd.push(' ');
        cmd.push('\'');
        cmd.push_str(self.args(jail, conf).join("' '").as_str());
        cmd.push('\'');
        cmd
    }
}
#[derive(Debug, Deserialize)]
pub struct Brand {
    modname: String,
    pub install: Step,
    pub init: Step,
    pub boot: Step,
    pub halt: Step,
    pub halted: Step,
    pub login: Step,
}


impl Brand {
    fn from_file(file: &str) -> Result<Self, Box<std::error::Error>> {
        let mut file = File::open(file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect(
            "Failed to read brand file.",
        );
        let result: Brand = toml::from_str(contents.as_str())?;
        Ok(result)
    }
    pub fn load(brand: &str, conf: &Config) -> Result<Self, Box<std::error::Error>> {
        let mut brand_root = PathBuf::from(conf.settings.brand_dir.as_str());
        brand_root.push(brand);
        brand_root.push("config.toml");
        Brand::from_file(brand_root.to_string_lossy().as_ref())
    }
}

#[cfg(test)]
mod tests {
    use brand::Brand;
    #[test]
    fn jail() {
        match Brand::from_file("rel/pkgng/deploy/usr/local/lib/brand/jail/config.toml") {
            Ok(_) => assert!(true),

            Err(e) => {
                println!("{}", e);
                assert!(false)
            }
        }
    }
    #[test]
    fn lx_jail() {
        match Brand::from_file("rel/pkgng/deploy/usr/local/lib/brand/lx-jail/config.toml") {
            Ok(_) => assert!(true),

            Err(e) => {
                println!("{}", e);
                assert!(false)
            }
        }
    }
}
