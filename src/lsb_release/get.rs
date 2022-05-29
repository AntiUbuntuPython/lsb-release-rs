use crate::lsb_release::imp::dist::{DistroInfo, lsb_version};

pub trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

struct LSBInfoGetter;

// replacement for /usr/share/pyshared/lsb_release.py
impl LSBInfo for LSBInfoGetter {
    fn id(&self) -> Option<String> {
        DistroInfo::get_distro_information().ok().and_then(|a| a.id)
    }

    fn description(&self) -> Option<String> {
        DistroInfo::get_distro_information().ok().and_then(|a| a.description)
    }

    fn release(&self) -> Option<String> {
        DistroInfo::get_distro_information().ok().and_then(|a| a.release)
    }

    fn codename(&self) -> Option<String> {
        DistroInfo::get_distro_information().ok().and_then(|a| a.codename)
    }

    // this is check_modules_installed()
    fn lsb_version(&self) -> Option<Vec<String>> {
        lsb_version()
    }
}

pub fn grub_info() -> impl LSBInfo {
    LSBInfoGetter
}