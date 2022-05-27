#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod lsb_release;
mod args;

use clap::Parser;
use crate::args::Args;
use crate::lsb_release::{LSBInfo, grub_info};

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
