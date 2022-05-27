use clap::Parser;

#[derive(Parser)]
struct Args {
    show_lsb_modules: bool,
    show_distributor: bool,
    show_description: bool,
    show_release: bool,
    show_codename: bool,
    show_all: bool,
    show_in_short_format: bool,
}

impl Args {
    fn set_implied_flags(mut self) -> Self {
        if self.show_all {
            self.show_lsb_modules = true;
            self.show_distributor = true;
            self.show_description = true;
            self.show_release = true;
            self.show_codename = true;
        }

        self
    }
}

fn main() {
    let args: Args = Args::parse();


}
