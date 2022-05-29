use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::env::{var, vars};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use voca_rs::Voca;

pub(crate) trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

struct LSBInfoGetter;

static modnamare: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"lsb-(?P<module>[a-z\d]+)-(?P<arch>[^ ]+)(?: \(= (?P<version>[\d.]+)\))?"#)
        .unwrap()
});

// replacement for /usr/share/pyshared/lsb_release.py
impl LSBInfo for LSBInfoGetter {
    fn id(&self) -> Option<String> {
        get_distro_information().ok().and_then(|a| a.id)
    }

    fn description(&self) -> Option<String> {
        get_distro_information().ok().and_then(|a| a.description)
    }

    fn release(&self) -> Option<String> {
        get_distro_information().ok().and_then(|a| a.release)
    }

    fn codename(&self) -> Option<String> {
        get_distro_information().ok().and_then(|a| a.codename)
    }

    // this is check_modules_installed()
    fn lsb_version(&self) -> Option<Vec<String>> {
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
            .envs(vars())
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
            let (version, provides) = (elements.get(0).unwrap(), elements.get(1).unwrap());
            // NOTE: `as_str` for arbitrary `for<'a> SplitN<'a, P: Pattern>` is unstable:
            //       it requires `str_split_as_str` as of 1.60.0
            let version = {
                // Debian revision splitter is one of them
                ['-', '+', '~']
                    .into_iter()
                    .find(|a| version.contains(*a))
                    .map_or(*version, |s| {
                        version.splitn(2, s).collect::<Vec<_>>().get(0).unwrap()
                    })
            };

            for pkg in provides.split(',') {
                let mob = modnamare.captures(pkg).unwrap();
                // no match
                if mob.is_none() {
                    continue;
                }

                let named_groups = mob.unwrap();

                let module = named_groups.name("module").unwrap().as_str();
                // false-positive
                #[allow(unused_variables)]
                let arch = named_groups.name("arch").unwrap().as_str();
                if named_groups.name("version").is_some() {
                    #[allow(unused_variables)]
                    let version = named_groups.name("version").unwrap().as_str();
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
        if module_vec.is_empty() {
            return None;
        }

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
        },
        _ => return vec![version],
    }
    .to_vec()
}

#[derive(Eq, PartialEq, Default, Clone)]
struct DistroInfo {
    release: Option<String>,
    codename: Option<String>,
    id: Option<String>,
    description: Option<String>,
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
}

// this is guess_debian_release()
fn guess_debian_release() -> Result<DistroInfo, Box<dyn Error>> {
    let mut lsbinfo = DistroInfo {
        id: Some("Debian".to_string()),
        ..DistroInfo::default()
    };
    let dpkg_origin = dpkg_origin();
    {
        // FIXME: this is not correct. should skip operation instead of panicking
        let f = File::open(dpkg_origin)
            .map_err(|e| eprintln!("Unable to open dpkg_origin: {e}"))
            .unwrap();
        let f = BufReader::new(f);
        let lines = f.lines().map(Result::unwrap).collect::<Vec<_>>();
        for line in lines {
            let elements = line.splitn(2, ": ").collect::<Vec<_>>();
            let (header, content) = (elements.get(0).unwrap(), elements.get(1).unwrap());
            let header = header._lower_case();
            let content = content.trim();
            if header == "vendor" {
                lsbinfo.id = Some(content.to_string());
            }
        }
    }

    let x = get_distro_info(lsbinfo.id.clone());

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
    lsbinfo.release = {
        let path = etc_debian_version();
        // FIXME: this is not correct. should skip operation instead of panicking
        let read_lines = &BufReader::new(
            File::open(path)
                .map_err(|e| eprintln!("Unable to open debian_release: {e}"))
                .unwrap(),
        )
        .lines()
        .collect::<Vec<_>>();

        let release = read_lines.get(0).unwrap().as_ref();

        // borrow checkers :c
        let unknown = &"unknown".to_string();
        let release = match release {
            Ok(release) => release,
            Err(_) => unknown,
        };

        if !&release[0..=1]._is_alpha() {
            let codename = lookup_codename(&x, release).unwrap_or_else(|| "n/a".to_string());
            lsbinfo.codename = Some(codename);
            Some(release.to_string())
        } else if release.ends_with("/sid") {
            let strip = release.strip_suffix("/sid").unwrap();
            let strip2 = strip.to_lowercase();
            if strip2 == "testing" {
                None
            } else {
                Some(strip.to_string())
            }
        } else {
            Some(release.to_string())
        }
    };

    if lsbinfo.codename.is_none() {
        let rinfo = guess_release_from_apt(None, None, None, None, None, &x);
        if let Some(mut rinfo) = rinfo {
            let release = {
                let release = rinfo.0.get("version");

                if let Some(release) = release {
                    if rinfo.0.get("origin").unwrap() == &"Debian Ports".to_string()
                        && ["ftp.ports.debian.org", "ftp.debian-ports.org"]
                            .contains(&rinfo.0.get("label").unwrap().as_str())
                    {
                        rinfo.0.insert("suite".to_string(), "unstable".to_string());
                        None
                    } else {
                        Some(release)
                    }
                } else {
                    release
                }
            };

            let codename = if let Some(release) = release {
                lookup_codename(&x, release)
            } else {
                let release = rinfo
                    .0
                    .get("suite")
                    .cloned()
                    .unwrap_or_else(|| "unstable".to_string());
                if release == "testing" {
                    x.debian_testing_codename
                } else {
                    Some("sid".to_string())
                }
            };

            lsbinfo.release = release.cloned();
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

fn guess_release_from_apt(
    origin: Option<String>,
    component: Option<String>,
    ignore_suites: Option<Vec<String>>,
    label: Option<String>,
    alternate_olabels_ports: Option<HashMap<String, Vec<String>>>,
    x: &X,
) -> Option<AptPolicy> {
    let releases = parse_apt_policy();
    let origin = origin.unwrap_or_else(|| "Debian".to_string());
    let component = component.unwrap_or_else(|| "main".to_string());
    let ignore_suites = ignore_suites.unwrap_or_else(|| vec!["experimental".to_string()]);
    let label = label.unwrap_or_else(|| "Debian".to_string());
    let alternate_olabels_ports = alternate_olabels_ports.unwrap_or_else(|| {
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

    if releases.as_ref().err().is_some() {
        return None;
    }

    let releases = releases.ok().unwrap();
    if releases.is_empty() {
        return None;
    }

    let mut dim = vec![];
    for release in releases {
        let policies = &release.policy.0;
        let p_origin = policies
            .get(&"policies".to_string())
            .cloned()
            .unwrap_or_default();
        let p_suite = policies
            .get(&"suite".to_string())
            .cloned()
            .unwrap_or_default();
        let p_component = policies
            .get(&"component".to_string())
            .cloned()
            .unwrap_or_default();
        let p_label = policies
            .get(&"label".to_string())
            .cloned()
            .unwrap_or_default();

        if p_origin == origin
            && !ignore_suites.contains(&p_suite)
            && p_component == component
            && p_label == label
            || (alternate_olabels_ports.contains_key(&p_origin)
                && alternate_olabels_ports
                    .get(&p_origin)
                    .unwrap()
                    .contains(&label))
        {
            dim.push(release);
        }
    }

    if dim.is_empty() {
        return None;
    }

    dim.sort_by_key(|a| a.priority);
    dim.reverse();

    let dim = dim;

    let max_priority = dim.get(0).unwrap().priority;
    let mut releases = dim
        .iter()
        .filter(|x| x.priority == max_priority)
        .collect::<Vec<_>>();
    releases.sort_by_key(|a| {
        let policy = a.policy.0.get(&*"suite".to_string());

        policy.map_or(0, |suite| {
            if x.release_order.contains(suite) {
                x.release_order.len() - x.release_order.iter().position(|a| a == suite).unwrap()
            } else {
                // FIXME: this is not correct in strict manner.
                suite.parse::<f64>().unwrap_or(0.0) as usize
            }
        })
    });

    Some(releases.get(0).copied().unwrap().clone().policy)
}

fn parse_apt_policy() -> Result<Vec<AptCachePolicyEntry>, Box<dyn Error>> {
    let mut data = vec![];
    let apt_cache_policy_output = Command::new("apt-cache")
        .arg("policy")
        .envs(vars().collect::<HashMap<_, _>>().tap_mut(|h| {
            h.insert("LC_ALL".to_string(), "C.UTF-8".to_string());
        }))
        .spawn()?
        .wait_with_output()?;

    // SAFETY: this shall be UTF-8

    let regex = Regex::new(r#"(-?\d+)"#).unwrap();

    for line in String::from_utf8(apt_cache_policy_output.stdout)
        .expect("This byte sequence is not valid UTF-8")
        .lines()
    {
        let line = line.trim();

        if line.starts_with("release") {
            let priority = regex
                .captures(line)
                .unwrap()
                .map(|c| c.get(1).unwrap())
                .map(|c1| c1.as_str().parse::<i64>().unwrap())
                .unwrap();
            let bits = line.splitn(2, ' ').collect::<Vec<_>>();
            if bits.len() > 1 {
                data.push(AptCachePolicyEntry {
                    priority,
                    policy: bits.get(1).unwrap().parse::<AptPolicy>().unwrap(),
                });
            }
        }
    }

    Ok(data)
}

#[derive(Eq, PartialEq, Clone)]
struct AptCachePolicyEntry {
    priority: i64,
    policy: AptPolicy,
}

#[derive(Eq, PartialEq, Clone)]
struct AptPolicy(HashMap<String, String>);

impl FromStr for AptPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let long_names = vec![
            ("v", "version"),
            ("o", "origin"),
            ("a", "suite"),
            ("c", "component"),
            ("l", "label"),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let bits = s.split(',').collect::<Vec<_>>();
        let mut hash_map = HashMap::with_capacity(bits.len());
        for bit in bits {
            let kv = bit.splitn(2, '=').collect::<Vec<_>>();
            if kv.len() > 1 {
                let (k, v) = (kv.get(0).unwrap(), kv.get(1).copied().unwrap());
                if let Some(kl) = long_names.get(k) {
                    hash_map.insert(kl, v);
                }
            }
        }

        Ok(Self(
            hash_map
                .into_iter()
                .map(|(k, v)| ((*k).to_string(), v.to_string()))
                .collect(),
        ))
    }
}

fn lookup_codename(x: &X, release: &str) -> Option<String> {
    let regex = Regex::new(r#"(\d+)\.(\d+)(r(\d+))?"#).unwrap();
    match regex.captures(release).unwrap() {
        None => None,
        Some(captures) => {
            let c1 = captures.get(1).unwrap().as_str().parse::<u32>().unwrap();
            let short = if c1 < 7 {
                format!("{c1}.{c2}", c2 = captures.get(2).unwrap().as_str())
            } else {
                format!("{c1}")
            };

            x.codename_lookup
                .iter()
                .find(|p| p.version == short)
                .map(|a| a.version.clone())
        }
    }
}

fn etc_debian_version() -> impl AsRef<Path> {
    var("LSB_ETC_DEBIAN_VERSION").unwrap_or_else(|_| "/etc/debian_version".to_string())
}

use serde::Deserialize;
use tap::Tap;

#[derive(Deserialize, Eq, PartialEq, Clone)]
struct DistroInfoCsvRecord {
    version: String,
    series: String,
}

#[derive(Eq, PartialEq)]
struct X {
    codename_lookup: Vec<DistroInfoCsvRecord>,
    release_order: Vec<String>,
    debian_testing_codename: Option<String>,
}

fn get_distro_info(origin: Option<String>) -> X {
    let origin = origin.unwrap_or_else(|| "Debian".to_string());
    let csv_file = get_distro_csv(origin.clone());

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

    let debian_testing_codename = if origin.to_lowercase() == *"debian" {
        release_order.append(&mut vec![
            "stable".to_string(),
            "proposed-updates".to_string(),
            "testing".to_string(),
            "testing-proposed-updates".to_string(),
            "unstable".to_string(),
            "sid".to_string(),
        ]);

        Some("unknown.new.testing")
    } else {
        None
    };

    X {
        codename_lookup,
        release_order,
        debian_testing_codename: debian_testing_codename.map(std::string::ToString::to_string),
    }
}

fn get_distro_csv(#[allow(unused_variables)] origin: String) -> impl AsRef<Path> {
    let path = format!("/usr/share/distro-info/{origin}.csv");
    if Path::new(&path).exists() {
        path
    } else {
        // fallback
        "/usr/share/distro-info/debian.csv".to_string()
    }
}

fn dpkg_origin() -> impl AsRef<Path> {
    var("LSB_ETC_DPKG_ORIGINS_DEFAULT").unwrap_or_else(|_| "/etc/dpkg/origins/default".to_string())
}

// this is get_os_release()
fn get_partial_info(path: impl AsRef<Path>) -> Result<DistroInfo, Box<dyn Error>> {
    File::open(path)
        .map(|read| {
            let read = BufReader::new(read);
            let unwraped = read.lines().map(Result::unwrap).collect::<Vec<_>>();
            let mut info = DistroInfo::default();
            for line4 in unwraped {
                let line = line4.as_str().trim();
                if line.is_empty() {
                    continue;
                }

                if !line.contains('=') {
                    continue;
                }

                let elements = line.splitn(2, '=').collect::<Vec<_>>();
                let (var, arg) = (elements.get(0).unwrap(), elements.get(1).unwrap());
                let arg = if arg.starts_with('"') && arg.ends_with('"') {
                    &arg[1..arg.len() - 1]
                } else {
                    arg
                };

                if arg.is_empty() {
                    continue;
                }

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
            info
        })
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

fn get_distro_information() -> Result<DistroInfo, Box<dyn Error>> {
    let lsbinfo = get_partial_info(get_path())?;
    if lsbinfo.is_partial() {
        let lsbinfo = lsbinfo.merged(&guess_debian_release()?);
        return Ok(lsbinfo);
    }

    Ok(lsbinfo)
}

fn get_path() -> impl AsRef<Path> {
    var("LSB_OS_RELEASE").unwrap_or_else(|_| "/usr/lib/os-release".to_string())
}

pub(crate) fn grub_info() -> impl LSBInfo {
    LSBInfoGetter
}
