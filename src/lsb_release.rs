use std::collections::HashSet;
use std::env::var;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;
use std::process::Command;
use fancy_regex::Regex;
use once_cell::sync::Lazy;

pub(crate) trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

struct LSBInfoGetter;

static modnamare: Lazy<Regex> = Lazy::new(|| Regex::new(r#"lsb-(?P<module>[a-z0-9]+)-(?P<arch>[^ ]+)(?: \(= (?P<version>[0-9.]+)\))?"#).unwrap());

// replacement for /usr/share/pyshared/lsb_release.py
impl LSBInfo for LSBInfoGetter {
    fn id(&self) -> Option<String> {
        todo!()
    }

    fn description(&self) -> Option<String> {
        todo!()
    }

    fn release(&self) -> Option<String> {
        todo!()
    }

    fn codename(&self) -> Option<String> {
        todo!()
    }

    // this is check_modules_installed()
    fn lsb_version(&self) -> Option<Vec<String>> {
        use std::env;
        let mut dpkg_query_args = vec![
            "-f".to_string(),
            // NOTE: this is dpkg-query formatter, no need to interpolate
            format!("${{Version}} ${{Provides}}\n"),
            "-W".to_string()
        ];

        // NOTE: this list may grow eventually!
        let mut packages = vec![
            "lsb-core".to_string(),
            "lsb-cxx".to_string(),
            "lsb-graphics".to_string(),
            "lsb-desktop".to_string(),
            "lsb-languages".to_string(),
            "lsb-multimedia".to_string(),
            "lsb-printing".to_string(),
            "lsb-security".to_string(),
        ];

        dpkg_query_args.append(&mut packages);

        let dpkg_query_result = Command::new("dpkg-query")
            .envs(env::vars())
            .args(dpkg_query_args)
            .spawn().ok()?
            .wait_with_output().ok()?;

        let query_result_lines = dpkg_query_result.stdout;
        if query_result_lines.is_empty() {
            return None
        }

        let mut modules = HashSet::new();
        for line in String::from_utf8(query_result_lines)
            .expect("It's not valid UTF-8").lines() {
            if line.is_empty() {
                continue
            }

            let elements = line.splitn(2, ' ').collect::<Vec<_>>();
            let (version, provides) = (elements.get(0).unwrap(), elements.get(1).unwrap());
            // NOTE: `as_str` for arbitrary `for<'a> SplitN<'a, P: Pattern>` is unstable:
            //       it requires `str_split_as_str` as of 1.60.0
            let version = {
                // Debian revision splitter is one of them
                ['-', '+', '~']
                    .into_iter()
                    .find(|a| version.contains(*a))
                    .map(|s| version.splitn(2, s).collect::<Vec<_>>().get(0).unwrap().clone())
                    .unwrap_or(version)
            };

            for pkg in provides.split(',') {
                let mob = modnamare.captures(pkg).unwrap();
                // no match
                if mob.is_none() {
                    continue
                }

                let named_groups = mob.unwrap();

                if named_groups.name("version").is_some() {
                    let module = named_groups.name("module").unwrap().as_str();
                    let version = named_groups.name("version").unwrap().as_str();
                    let arch = named_groups.name("arch").unwrap().as_str();

                    let module = format!("{module}s-{version}s-{arch}s");
                    modules.insert(module);
                } else {
                    let module = named_groups.name("module").unwrap().as_str();
                    for v in valid_lsb_versions(version, module) {
                        let module = named_groups.name("module").unwrap().as_str();
                        let version = v;
                        let arch = named_groups.name("arch").unwrap().as_str();

                        let module = format!("{module}s-{version}s-{arch}s");
                        modules.insert(module);
                    }
                }
            }

        }
        let mut module_vec = modules.into_iter().collect::<Vec<_>>();
        module_vec.sort();
        Some(module_vec)
    }
}

fn valid_lsb_versions<'v: 'r, 'r>(version: &'v str, module: &'r str) -> Vec<&'r str> {
    match version {
        "3.0" => &["2.0", "3.0"] as &[&str],
        "3.1" => match module {
            "desktop" | "qt4" => &["3.1"] as &[&str],
            "cxx" => &["3.0"],
            _ => &["2.0", "3.0", "3.1"],
        },
        "3.2" => match module {
            "desktop" => &["3.1", "3.2"] as &[&str],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2"],
            "cxx" => &["3.0", "3.1", "3.2"],
            _ => &["2.0", "3.0", "3.1", "3.2"],
        },
        "4.0" => match module {
            "desktop" => &["3.1", "3.2", "4.0"] as &[&str],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2", "4.0"],
            "security" => &["4.0"],
            "cxx" => &["3.0", "3.1", "3.2", "4.0"],
            _ => &["2.0", "3.0", "3.1", "3.2", "4.0"],
        },
        "4.1" => match module {
            "desktop" => &["3.1", "3.2", "4.0", "4.1"] as &[&str],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2", "4.0", "4.1"],
            "security" => &["4.0", "4.1"],
            "cxx" => &["3.0", "3.1", "3.2", "4.0", "4.1"],
            _ => &["2.0", "3.0", "3.1", "3.2", "4.0", "4.1"],
        }
        _ => return vec![version.clone()]
    }.to_vec()
}

#[derive(Eq, PartialEq, Default)]
struct DistroInfo {
    release: Option<String>,
    codename: Option<String>,
    id: Option<String>,
    description: Option<String>,
}

impl DistroInfo {
    fn is_partial(&self) -> bool {
        self.release.is_none() && self.codename.is_none() && self.id.is_none() && self.description.is_none()
    }

    fn merged(&self, other: Self) -> Self {
        Self {
            release: self.release.as_ref().or(other.release.as_ref()).cloned(),
            codename: self.codename.as_ref().or(other.codename.as_ref()).cloned(),
            id: self.id.as_ref().or(other.id.as_ref()).cloned(),
            description: self.description.as_ref().or(other.description.as_ref()).cloned(),
        }
    }
}

// this is guess_debian_release()
fn guess_debian_release() -> Result<DistroInfo, Box<dyn Error>> {
    let mut lsbinfo = DistroInfo::default();
    lsbinfo.id = Some("Debian".to_string());

    let dpkg_origin = dpkg_origin();


    Ok(lsbinfo)
}

fn dpkg_origin() -> impl AsRef<Path> {
    var("LSB_ETC_DPKG_ORIGINS_DEFAULT").unwrap_or("/etc/dpkg/origins/default".to_string())
}

// this is get_os_release()
fn get_partial_info(path: impl AsRef<Path>) -> Result<DistroInfo, Box<dyn Error>> {
    File::open(path).map(|read| {
        let read = BufReader::new(read);
        let unwraped = read.lines().map(|a| a.unwrap()).collect::<Vec<_>>();
        for line4 in unwraped {
            let line = line4.as_str().trim();
            if line.is_empty() {
                continue
            }

            if !line.contains('=') {
                continue
            }

            let mut info = DistroInfo::default();
            let elements = line.splitn(2, '=').collect::<Vec<_>>();
            let (var, arg) = (elements.get(0).unwrap(), elements.get(1).unwrap());
            let arg = if arg.starts_with('"') && arg.ends_with('"') {
                &arg[1..arg.len()-1]
            } else {
                arg
            };

            if arg.is_empty() {
                continue
            }

            use voca_rs::Voca;

            match *var {
                "VERSION_ID" => {
                    info.release = Some(arg.trim().to_string());
                }
                "VERSION_CODENAME" => {
                    info.codename = Some(arg.trim().to_string());
                }
                "ID" => {
                    info.id = Some(arg.trim()._title_case().to_string());
                }
                "PRETTY_NAME" => {
                    info.description = Some(arg.trim().to_string());
                }

                _ => {}
            }
        }

        todo!("shut up type error")
    }).map_err(|e| Box::new(e) as Box<dyn Error>)

}

fn get_distro_information() -> Result<DistroInfo, Box<dyn Error>> {
    let lsbinfo = get_partial_info(get_path())?;
    if lsbinfo.is_partial() {
        let lsbinfo = lsbinfo.merged(guess_debian_release()?);
        return Ok(lsbinfo)
    }

    Ok(lsbinfo)
}

fn get_path() -> impl AsRef<Path> {
    std::env::var("LSB_OS_RELEASE").unwrap_or("/usr/lib/os-release".to_string())
}

pub(crate) fn grub_info() -> impl LSBInfo {
    LSBInfoGetter
}
