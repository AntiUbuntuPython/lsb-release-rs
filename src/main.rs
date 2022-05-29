#![deny(clippy::all, clippy::string_to_string)]
#![warn(clippy::pedantic, clippy::nursery, clippy::restriction, clippy::cargo, clippy::get_unwrap)]
#![allow(
    clippy::cargo_common_metadata,
    clippy::blanket_clippy_restriction_lints,
    clippy::missing_docs_in_private_items,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::shadow_reuse,
    clippy::implicit_return,
    clippy::str_to_string,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::integer_arithmetic,
    clippy::string_slice,
    clippy::unwrap_in_result,
    clippy::expect_used,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::default_numeric_fallback,
)]

mod args;
mod lsb_release;

use crate::args::Args;
use crate::lsb_release::get::{grub_info, LSBInfo};
use clap::Parser;

fn main() {
    let args: Args = Args::parse();
    let args = args.set_implied_flags();
    let grub = grub_info();
    let short = args.show_in_short_format;
    let na = "n/a".to_string();

    if args.show_lsb_modules {
        match grub.lsb_version() {
            None => {
                eprintln!("No LSB modules are available.");
            }
            Some(lsb_version) => {
                let v = lsb_version.join(":");
                if short {
                    println!("{v}");
                } else {
                    println!("LSB Version: \t{v}");
                }
            }
        }
    }

    if args.show_distributor {
        let v = grub.id().unwrap_or_else(|| na.clone());

        if short {
            println!("{v}");
        } else {
            println!("Distributor ID:\t{v}");
        }
    }

    if args.show_description {
        let v = &grub.description().unwrap_or_else(|| na.clone());

        if short {
            println!("{v}");
        } else {
            println!("Description:\t{v}");
        }
    }

    if args.show_release {
        let v = &grub.release().unwrap_or_else(|| na.clone());

        if short {
            println!("{v}");
        } else {
            println!("Release:\t{v}");
        }
    }

    if args.show_codename {
        let v = &grub.codename().unwrap_or_else(|| na.clone());

        if short {
            println!("{v}");
        } else {
            println!("Codename:\t{v}");
        }
    }
}
