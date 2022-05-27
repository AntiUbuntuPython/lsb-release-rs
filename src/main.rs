#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short = 'v', long = "version", long = "show-lsb-modulesw")]
    show_lsb_modules: bool,
    #[clap(short = 'i', long = "id")]
    show_distributor: bool,
    #[clap(short = 'd', long = "description")]
    show_description: bool,
    #[clap(short = 'r', long = "release")]
    show_release: bool,
    #[clap(short = 'c', long = "codename")]
    show_codename: bool,
    #[clap(short = 'a', long = "all")]
    show_all: bool,
    #[clap(short = 's', long = "short")]
    show_in_short_format: bool,
}

impl Args {
    const fn set_implied_flags(mut self) -> Self {
        if self.show_all {
            self.show_lsb_modules = true;
            self.show_distributor = true;
            self.show_description = true;
            self.show_release = true;
            self.show_codename = true;
        } else {
            self.show_lsb_modules = !self.show_lsb_modules && !self.show_distributor && !self.show_description && !self.show_release && !self.show_codename;
        }

        self
    }
}

trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

fn grub_info() -> impl LSBInfo {
    todo!("not yet")
}

fn main() {
    let args: Args = Args::parse();
    let args = args.set_implied_flags();
    let grub = grub_info();
    let short = args.show_in_short_format;
    let na = "n/a".to_string();

    if args.show_lsb_modules {
        match grub.lsb_version() {
            None => {
                eprintln!("No LSB modules are available.")
            }
            Some(lsb_version) => {
                let v = lsb_version.join(":");
                if short {
                    println!("{v}")
                } else {
                    println!("LSB Version: \t{v}")
                }
            }
        }
    }

    if args.show_distributor {
        let v = grub.id().unwrap_or(na.clone());

        if short {
            println!("{v}")
        } else {
            println!("Distributor ID:\t{v}")
        }
    }

    if args.show_description {
        let v = &grub.description().unwrap_or(na.clone());

        if short {
            println!("{v}")
        } else {
            println!("Description:\t{v}")
        }
    }

    if args.show_release {
        let v = &grub.release().unwrap_or(na.clone());

        if short {
            println!("{v}")
        } else {
            println!("Release:\t{v}")
        }
    }

    if args.show_codename {
        let v = &grub.codename().unwrap_or(na.clone());

        if short {
            println!("{v}")
        } else {
            println!("Codename:\t{v}")
        }
    }
}
