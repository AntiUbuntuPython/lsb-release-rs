use std::collections::{HashMap, HashSet};
use std::env::var;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use serde::Deserialize;
use voca_rs::Voca;
use crate::lsb_release::imp::apt::{AptPolicy, dpkg_default_vendor, parse_apt_policy};
use crate::lsb_release::imp::lsb::valid_lsb_versions;

static MOD_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"lsb-(?P<module>[a-z\d]+)-(?P<arch>[^ ]+)(?: \(= (?P<version>[\d.]+)\))?"#)
        .unwrap()
});

#[derive(Eq, PartialEq, Default, Clone)]
pub(in crate::lsb_release) struct DistroInfo {
    pub(in crate::lsb_release) release: Option<String>,
    pub(in crate::lsb_release) codename: Option<String>,
    pub(in crate::lsb_release) id: Option<String>,
    pub(in crate::lsb_release) description: Option<String>,
}

#[derive(Default, Eq, PartialEq, Debug)]
struct Y {
    release: Option<String>,
    codename: Option<String>,
}

fn get_debian_release(x: &X) -> Result<Y, Box<dyn Error>> {
    let path = PathGetter::debian_version();
    let read_lines = &BufReader::new(
        File::open(path)?,
    )
        .lines()
        .collect::<Vec<_>>();

    let release = read_lines[0].as_ref();

    // borrow checkers :c
    let unknown = &"unknown".to_string();
    let release = release.unwrap_or(unknown);

    let mut y = Y::default();

    if !&release[0..=1]._is_alpha() {
        let codename = x.lookup_codename(release).unwrap_or_else(|| "n/a".to_string());
        y.codename = Some(codename);
        y.release = Some(release.to_string());
    } else if release.ends_with("/sid") {
        let strip = release.strip_suffix("/sid").unwrap();
        y.release = (strip.to_lowercase() != "testing").then(|| strip.to_string())
    } else {
        y.release = Some(release.to_string())
    };

    Ok(y)
}

impl DistroInfo {
    const fn is_partial(&self) -> bool {
        self.release.is_none()
            && self.codename.is_none()
            && self.id.is_none()
            && self.description.is_none()
    }

    fn merged(&self, other: &Self) -> Self {
        Self {
            release: self.release.as_ref().or(other.release.as_ref()).cloned(),
            codename: self.codename.as_ref().or(other.codename.as_ref()).cloned(),
            id: self.id.as_ref().or(other.id.as_ref()).cloned(),
            description: self
                .description
                .as_ref()
                .or(other.description.as_ref())
                .cloned(),
        }
    }


    // this is guess_debian_release()
    fn guess_debian_release() -> Result<Self, Box<dyn Error>> {
        let mut lsbinfo = Self {
            id: Some("Debian".to_string()),
            ..DistroInfo::default()
        };
        lsbinfo.id = dpkg_default_vendor().ok().flatten();

        let x = X::get_distro_info(lsbinfo.id.clone());

        #[allow(unused_variables)]
            let os = match uname_rs::Uname::new()?.sysname.as_str() {
            #[allow(unused_variables)]
            x @ ("Linux" | "Hurd" | "NetBSD") => format!("GNU/{x}"),
            "FreeBSD" => "GNU/kFreeBSD".to_string(),
            x @ ("GNU/Linux" | "GNU/kFreeBSD") => x.to_string(),
            _ => "GNU".to_string(),
        };

        lsbinfo.description = Some(format!(
            "{id}s {os}s",
            id = lsbinfo.id.clone().unwrap_or_default()
        ));
        let y = get_debian_release(&x)?;
        lsbinfo.release = y.release;
        lsbinfo.codename = y.codename;

        if lsbinfo.codename.is_none() {
            let rinfo = x.guess_release_from_apt(None, None, None, None, None);
            if let Some(mut rinfo) = rinfo {
                let release = rinfo.version.and_then(|release| {
                    let condition = rinfo.origin.unwrap() == *"Debian Ports"
                        && ["ftp.ports.debian.org", "ftp.debian-ports.org"]
                        .contains(&rinfo.label.unwrap().as_str());

                    if condition {
                        rinfo.suite = Some("unstable".to_string());
                    }

                    (!condition).then(|| release)
                });

                let codename = release.clone().map_or_else(
                    || {
                        let release = rinfo.suite
                            .unwrap_or_else(|| "unstable".to_string());
                        if release == "testing" {
                            x.debian_testing_codename.clone()
                        } else {
                            Some("sid".to_string())
                        }
                    },
                    |release| x.lookup_codename(release.as_str())
                );

                lsbinfo.release = release;
                lsbinfo.codename = codename;
            }
        }

        if let Some(ref release) = lsbinfo.release {
            lsbinfo.description = lsbinfo.description.map(|d| format!("{d} {release}"));
        }

        if let Some(ref codename) = lsbinfo.codename {
            lsbinfo.description = lsbinfo.description.map(|d| format!("{d} {codename}"));
        }

        Ok(lsbinfo)
    }

    // this is get_os_release()
    fn get_partial_info(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        File::open(path)
            .map(|read| {
                let read = BufReader::new(read);
                let unwraped = read.lines();
                let mut info = DistroInfo::default();
                for line4 in unwraped {
                    let line23 = line4.unwrap();
                    let line = line23.as_str().trim();
                    if line.is_empty() {
                        continue;
                    }

                    if !line.contains('=') {
                        continue;
                    }

                    let elements = line.splitn(2, '=').collect::<Vec<_>>();
                    let (var, arg) = (elements[0], elements[1]);
                    let arg = if arg.starts_with('"') && arg.ends_with('"') {
                        &arg[1..arg.len() - 1]
                    } else {
                        arg
                    };

                    if arg.is_empty() {
                        continue;
                    }

                    match var {
                        "VERSION_ID" => {
                            info.release = Some(arg.trim().to_string());
                        }
                        "VERSION_CODENAME" => {
                            info.codename = Some(arg.trim().to_string());
                        }
                        "ID" => {
                            info.id = Some(arg.trim()._title_case());
                        }
                        "PRETTY_NAME" => {
                            info.description = Some(arg.trim().to_string());
                        }

                        _ => {}
                    }
                }
                info
            })
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    pub(in crate::lsb_release) fn get_distro_information() -> Result<Self, Box<dyn Error>> {
        let lsbinfo = Self::get_partial_info(PathGetter::lsb_os_release())?;
        if lsbinfo.is_partial() {
            let lsbinfo = lsbinfo.merged(&Self::guess_debian_release()?);
            return Ok(lsbinfo);
        }

        Ok(lsbinfo)
    }
}

#[derive(Eq, PartialEq)]
struct X {
    codename_lookup: Vec<DistroInfoCsvRecord>,
    release_order: Vec<String>,
    debian_testing_codename: Option<String>,
}

impl X {
    fn guess_release_from_apt(
        &self,
        origin: Option<String>,
        component: Option<String>,
        ignore_suites: Option<Vec<String>>,
        label: Option<String>,
        alternate_ports: Option<HashMap<String, Vec<String>>>,
    ) -> Option<AptPolicy> {
        let releases = parse_apt_policy();
        let origin = origin.unwrap_or_else(|| "Debian".to_string());
        let component = component.unwrap_or_else(|| "main".to_string());
        let ignore_suites = ignore_suites.unwrap_or_else(|| vec!["experimental".to_string()]);
        let label = label.unwrap_or_else(|| "Debian".to_string());
        let alternate_olabels_ports = alternate_ports.unwrap_or_else(|| {
            [(
                "Debian Ports".to_string(),
                vec![
                    "ftp.ports.debian.org".to_string(),
                    "ftp.debian-ports.org".to_string(),
                ],
            )]
                .into_iter()
                .collect()
        });

        let releases = releases.as_ref().ok()?;

        if releases.is_empty() {
            return None;
        }

        let dim = {
            let mut dim = releases.iter().filter(|release| {
                let p_origin = release.policy.origin
                    .clone()
                    .unwrap_or_default();
                let p_suite = release.policy.suite
                    .clone()
                    .unwrap_or_default();
                let p_component = release.policy.component
                    .clone()
                    .unwrap_or_default();
                let p_label = release.policy.label
                    .clone()
                    .unwrap_or_default();

                p_origin == origin
                    && !ignore_suites.contains(&p_suite)
                    && p_component == component
                    && p_label == label
                    || (alternate_olabels_ports.contains_key(&p_origin)
                    && alternate_olabels_ports[&p_origin].contains(&label))
            }).collect::<Vec<_>>();

            if dim.is_empty() {
                return None;
            }

            dim.sort_by_key(|a| std::cmp::Reverse(a.priority));

            dim
        };

        let max_priority = dim[0].priority;
        let mut releases = dim
            .iter()
            .filter(|x| x.priority == max_priority)
            .collect::<Vec<_>>();
        releases.sort_by_key(|a| {
            let policy = a.policy.suite.as_ref();

            policy.map_or(0, |suite| {
                if self.release_order.contains(suite) {
                    // NOTE: do you think you can contain 2^63 elements in your memory?
                    (self.release_order.len() - self.release_order.iter().position(|a| a == suite).unwrap()) as isize
                } else {
                    // FIXME: this is not correct in strict manner.
                    suite.parse::<f64>().unwrap_or(0.0) as isize
                }
            })
        });

        Some(releases[0].policy.clone())
    }

    fn lookup_codename(&self, release: &str) -> Option<String> {
        let regex = Regex::new(r#"(\d+)\.(\d+)(r(\d+))?"#).unwrap();
        regex.captures(release).unwrap().and_then(|captures| {
            let c1 = captures[1].parse::<u32>().unwrap();
            let short = if c1 < 7 {
                format!("{c1}.{c2}", c2 = &captures[2])
            } else {
                format!("{c1}")
            };

            self.codename_lookup
                .iter()
                .find(|p| p.version == short)
                .map(|a| a.version.clone())
        })
    }

    fn get_distro_info(origin: Option<String>) -> Self {
        let origin = origin.unwrap_or_else(|| "Debian".to_string());
        let csv_file = PathGetter::distro_info_csv(origin.as_str());

        let mut codename_lookup = csv::Reader::from_path(csv_file)
            .unwrap()
            .deserialize::<DistroInfoCsvRecord>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        // f64 is not Ord
        codename_lookup.sort_by(|a, b| {
            a.series
                .parse::<f64>()
                .unwrap()
                .partial_cmp(&b.series.parse::<f64>().unwrap())
                .unwrap()
        });
        let mut release_order = codename_lookup
            .iter()
            .map(|a| a.series.clone())
            .collect::<Vec<_>>();

        let debian_testing_codename = (origin.to_lowercase() == *"debian").then(|| {
            release_order.append(&mut vec![
                "stable".to_string(),
                "proposed-updates".to_string(),
                "testing".to_string(),
                "testing-proposed-updates".to_string(),
                "unstable".to_string(),
                "sid".to_string(),
            ]);

            "unknown.new.testing"
        });

        Self {
            codename_lookup,
            release_order,
            debian_testing_codename: debian_testing_codename.map(std::string::ToString::to_string),
        }
    }
}

struct PathGetter;

impl PathGetter {
    fn lsb_os_release() -> impl AsRef<Path> {
        var("LSB_OS_RELEASE").unwrap_or_else(|_| "/usr/lib/os-release".to_string())
    }

    fn distro_info_csv(#[allow(unused_variables)] origin: &str) -> impl AsRef<Path> {
        let path = format!("/usr/share/distro-info/{origin}.csv");
        if Path::new(&path).exists() {
            path
        } else {
            // fallback
            "/usr/share/distro-info/debian.csv".to_string()
        }
    }

    fn debian_version() -> impl AsRef<Path> {
        var("LSB_ETC_DEBIAN_VERSION").unwrap_or_else(|_| "/etc/debian_version".to_string())
    }
}

pub(in crate::lsb_release) fn lsb_version() -> Option<Vec<String>> {
    let mut dpkg_query_args = vec![
        "-f".to_string(),
        // NOTE: this is dpkg-query formatter, no need to interpolate
        format!("${{Version}} ${{Provides}}\n"),
        "-W".to_string(),
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

    #[allow(unused_variables)]
        let dpkg_query_result = Command::new("dpkg-query")
        .args(dpkg_query_args)
        // void dpkg-query error, such as "no such package"
        .stderr(Stdio::null())
        // don't inherit
        .stdout(Stdio::piped())
        .spawn()
        .ok()?
        .wait_with_output()
        .ok()?;

    let query_result_lines = dpkg_query_result.stdout;
    if query_result_lines.is_empty() {
        return None;
    }

    let mut modules = HashSet::new();
    for line in String::from_utf8(query_result_lines)
        .expect("It's not valid UTF-8")
        .lines()
    {
        if line.is_empty() {
            continue;
        }

        let elements = line.splitn(2, ' ').collect::<Vec<_>>();
        let (version, provides) = (elements[0], elements[1]);
        // NOTE: `as_str` for arbitrary `for<'a> SplitN<'a, P: Pattern>` is unstable:
        //       it requires `str_split_as_str` as of 1.60.0
        let version = {
            // Debian revision splitter is one of them
            ['-', '+', '~']
                .into_iter()
                .find(|a| version.contains(*a))
                .map_or(version, |s| {
                    version.splitn(2, s).collect::<Vec<_>>()[0]
                })
        };

        for pkg in provides.split(',') {
            let named_groups = match MOD_NAME_REGEX.captures(pkg).unwrap() {
                None => continue,
                Some(captures) => captures
            };

            let module = &named_groups["module"];
            // false-positive
            #[allow(unused_variables)]
                let arch = &named_groups["arch"];
            if named_groups.name("version").is_some() {
                #[allow(unused_variables)]
                    let version = &named_groups["version"];
                let module = format!("{module}s-{version}s-{arch}s");

                modules.insert(module);
            } else {
                for v in valid_lsb_versions(version, module) {
                    #[allow(unused_variables)]
                        let version = v;
                    let module = format!("{module}s-{version}s-{arch}s");

                    modules.insert(module);
                }
            }
        }
    }

    let mut module_vec = modules.into_iter().collect::<Vec<_>>();
    (!module_vec.is_empty()).then(|| {
        module_vec.sort();
        module_vec
    })
}

#[derive(Deserialize, Eq, PartialEq, Clone)]
struct DistroInfoCsvRecord {
    version: String,
    series: String,
}
