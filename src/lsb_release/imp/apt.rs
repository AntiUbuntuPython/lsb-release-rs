use std::env::var;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use fancy_regex::Regex;

#[derive(Eq, PartialEq, Clone)]
pub(in crate::lsb_release::imp) struct AptCachePolicyEntry {
    pub(in crate::lsb_release::imp) priority: i64,
    pub(in crate::lsb_release::imp) policy: AptPolicy,
}

#[derive(Eq, PartialEq, Clone, Default)]
pub(in crate::lsb_release::imp) struct AptPolicy {
    pub(in crate::lsb_release::imp) version: Option<String>,
    pub(in crate::lsb_release::imp) origin: Option<String>,
    pub(in crate::lsb_release::imp) suite: Option<String>,
    pub(in crate::lsb_release::imp) component: Option<String>,
    pub(in crate::lsb_release::imp) label: Option<String>,
}

impl FromStr for AptPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ret = Self::default();

        for bit in s.split(',') {
            let kv = bit.splitn(2, '=').collect::<Vec<_>>();
            if kv.len() > 1 {
                let (k, v) = (kv[0], kv[1]);
                match k {
                    "v" => {
                        ret.version = Some(v.to_string());
                    }
                    "o" => {
                        ret.origin = Some(v.to_string());
                    }
                    "a" => {
                        ret.suite = Some(v.to_string());
                    }
                    "c" => {
                        ret.component = Some(v.to_string());
                    }
                    "l" => {
                        ret.label = Some(v.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(ret)
    }
}

pub(in crate::lsb_release::imp) fn dpkg_default_vendor() -> Result<Option<String>, Box<dyn Error>> {
    let f = File::open(dpkg_origin())?;
    let f = BufReader::new(f);
    Ok(
        f.lines().map(Result::unwrap)
            .map(|line| line.splitn(2, ": ").map(std::string::ToString::to_string).collect::<Vec<_>>())
            .map(|elements| (elements[0].clone(), elements[1].clone()))
            .map(|(header, content)| (header.to_lowercase(), content.trim().to_string()))
            .filter(|(header, _)| header == "vendor")
            .last()
            .map(|(_, content)| content)
    )
}

fn dpkg_origin() -> impl AsRef<Path> {
    var("LSB_ETC_DPKG_ORIGINS_DEFAULT").unwrap_or_else(|_| "/etc/dpkg/origins/default".to_string())
}

pub(in crate::lsb_release::imp) fn parse_apt_policy() -> Result<Vec<AptCachePolicyEntry>, Box<dyn Error>> {
    let apt_cache_policy_output = Command::new("apt-cache")
        .arg("policy")
        // Command::new inherits env vars, so we need to just overwrite single variable
        .env("LC_ALL", "C.UTF-8")
        .spawn()?
        .wait_with_output()?;

    // SAFETY: this shall be UTF-8

    let regex = Regex::new(r#"(-?\d+)"#).unwrap();

    let data = String::from_utf8(apt_cache_policy_output.stdout)
        .expect("This byte sequence is not valid UTF-8")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("release"))
        .map(|line| {
            let priority = regex
                .captures(line)
                .unwrap()
                .map(|c| c[1].parse().unwrap())
                .unwrap();
            let bits = line.splitn(2, ' ').collect::<Vec<_>>();

            (priority, bits)
        })
        .filter(|(_, bits)| bits.len() > 1)
        .map(|(priority, bits)| AptCachePolicyEntry {
            priority,
            policy: bits[1].parse().unwrap()
        })
        .collect();

    Ok(data)
}
