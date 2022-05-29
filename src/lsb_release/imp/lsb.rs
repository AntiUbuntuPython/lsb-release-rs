pub(in crate::lsb_release::imp) fn valid_lsb_versions<'v: 'r, 'r>(
    version: &'v str,
    module: &'r str,
) -> Vec<&'r str> {
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
