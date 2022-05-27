use std::collections::HashSet;
use std::process::Command;
use fancy_regex::Regex;

pub(crate) trait LSBInfo {
    fn id(&self) -> Option<String>;

    fn description(&self) -> Option<String>;

    fn release(&self) -> Option<String>;

    fn codename(&self) -> Option<String>;

    fn lsb_version(&self) -> Option<Vec<String>>;
}

struct LSBInfoGetter;

static modnamare: Regex = Regex::new(r#"lsb-(?P<module>[a-z0-9]+)-(?P<arch>[^ ]+)(?: \(= (?P<version>[0-9.]+)\))?"#).unwrap();

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
            "-f",
            // NOTE: this is dpkg-query formatter, no need to interpolate
            format!("${{Version}} ${{Provides}}\n").as_str(),
            "-W"
        ];

        // NOTE: this list may grow eventually!
        let mut packages = vec![
            "lsb-core",
            "lsb-cxx",
            "lsb-graphics",
            "lsb-desktop",
            "lsb-languages",
            "lsb-multimedia",
            "lsb-printing",
            "lsb-security"
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
                    .find(|a| version.contains(a))
                    .map(|s| version.splitn(2, s).collect::<Vec<_>>().get(0).unwrap())
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

fn valid_lsb_versions<'b: 'a, 'a>(version: &'a str, module: &'a str) -> &'a [&'b str] {
    match version {
        "3.0" => &["2.0", "3.0"],
        "3.1" => match module {
            "desktop" | "qt4" => &["3.1"],
            "cxx" => &["3.0"],
            _ => &["2.0", "3.0", "3.1"],
        },
        "3.2" => match module {
            "desktop" => &["3.1", "3.2"],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2"],
            "cxx" => &["3.0", "3.1", "3.2"],
            _ => &["2.0", "3.0", "3.1", "3.2"],
        },
        "4.0" => match module {
            "desktop" => &["3.1", "3.2", "4.0"],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2", "4.0"],
            "security" => &["4.0"],
            "cxx" => &["3.0", "3.1", "3.2", "4.0"],
            _ => &["2.0", "3.0", "3.1", "3.2", "4.0"],
        },
        "4.1" => match module {
            "desktop" => &["3.1", "3.2", "4.0", "4.1"],
            "qt4" => &["3.1"],
            "printing" | "languages" | "multimedia" => &["3.2", "4.0", "4.1"],
            "security" => &["4.0", "4.1"],
            "cxx" => &["3.0", "3.1", "3.2", "4.0", "4.1"],
            _ => &["2.0", "3.0", "3.1", "3.2", "4.0", "4.1"],
        }
        _ => &[version.to_string().as_str()],
    }
}

pub(crate) fn grub_info() -> impl LSBInfo {
    LSBInfoGetter
}
