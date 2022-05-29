use clap::Parser;

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser)]
pub(crate) struct Args {
    #[clap(short = 'v', long = "version", long = "show-lsb-modulesw")]
    pub(crate) show_lsb_modules: bool,
    #[clap(short = 'i', long = "id")]
    pub(crate) show_distributor: bool,
    #[clap(short = 'd', long = "description")]
    pub(crate) show_description: bool,
    #[clap(short = 'r', long = "release")]
    pub(crate) show_release: bool,
    #[clap(short = 'c', long = "codename")]
    pub(crate) show_codename: bool,
    #[clap(short = 'a', long = "all")]
    show_all: bool,
    #[clap(short = 's', long = "short")]
    pub(crate) show_in_short_format: bool,
}

impl Args {
    pub(crate) const fn set_implied_flags(mut self) -> Self {
        if self.show_all {
            self.show_lsb_modules = true;
            self.show_distributor = true;
            self.show_description = true;
            self.show_release = true;
            self.show_codename = true;
        } else {
            self.show_lsb_modules = !self.show_lsb_modules
                && !self.show_distributor
                && !self.show_description
                && !self.show_release
                && !self.show_codename;
        }

        self
    }
}
