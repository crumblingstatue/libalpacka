#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use {
    alpacka::{Depend, OptDepend, Pkg, PkgDesc, ReqCmp, Validation, dep::PkgDepsExt},
    smol_str::SmolStr,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(op) = args.next() else {
        eprintln!("Need operation");
        return;
    };
    let pkg_name = args.next();
    let tz = jiff::tz::TimeZone::system();
    match op.as_str() {
        "-Qi" => {
            let mut pkgs = alpacka::read_local_db().unwrap();
            pkgs.sort_by_key(|pkg| pkg.desc.name.clone());
            let Some(pkg_name) = &pkg_name else {
                for pkg in &pkgs {
                    pkg_info_dump(&pkg.desc, &pkgs, &tz, None);
                    println!();
                }
                return;
            };
            match pkgs.iter().find(|pkg| pkg.desc.name == pkg_name) {
                Some(pkg) => {
                    pkg_info_dump(&pkg.desc, &pkgs, &tz, None);
                }
                None => {
                    eprintln!("No such package.");
                }
            }
        }
        "-Ql" => {
            let mut pkgs = alpacka::read_local_db().unwrap();
            pkgs.sort_by_key(|pkg| pkg.desc.name.clone());
            for pkg in &pkgs {
                for file in &pkg.files {
                    println!("{pkg} /{file}", pkg = pkg.desc.name);
                }
            }
        }
        "-Si" => {
            for db in [
                "core-testing",
                "core",
                "extra-testing",
                "extra",
                "multilib-testing",
                "multilib",
            ] {
                let mut pkgs = alpacka::read_syncdb(db).unwrap();
                pkgs.sort_by_key(|pkg| pkg.desc.name.clone());
                let Some(pkg_name) = &pkg_name else {
                    for pkg in &pkgs {
                        pkg_info_dump(&pkg.desc, &pkgs, &tz, Some(db));
                        println!();
                    }
                    continue;
                };
                match pkgs.iter().find(|pkg| pkg.desc.name == pkg_name) {
                    Some(pkg) => {
                        pkg_info_dump(&pkg.desc, &pkgs, &tz, Some(db));
                    }
                    None => {
                        eprintln!("No such package.");
                    }
                }
                println!();
            }
        }
        _ => {
            eprintln!("Unknown op");
        }
    }
}

#[expect(clippy::cast_precision_loss)]
fn pacman_humanize_size(bytes: u64, target_unit: u8, precision: i8) -> (f64, &'static str) {
    let labels = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let mut val = bytes as f64;
    let mut ret_label = labels[0];
    for label in labels {
        ret_label = label;
        if label.as_bytes()[0] == target_unit
            || (target_unit == 0 && (-2048.0..2048.0).contains(&val))
        {
            break;
        }
        val /= 1024.0;
    }
    if precision >= 0 && val < 0.0 && val > (-0.5 / 10f64.powi(precision.into())) {
        val = 0.0;
    }
    (val, ret_label)
}

fn pkg_info_dump(pkg: &PkgDesc, pkgs: &[Pkg], tz: &jiff::tz::TimeZone, syncdb: Option<&str>) {
    if let Some(syncdb) = syncdb {
        println!("Repository      : {}", syncdb);
    }
    println!("Name            : {}", pkg.name);
    println!("Version         : {}", pkg.version);
    if let Some(desc) = &pkg.desc {
        println!("Description     : {desc}");
    }
    println!("Architecture    : {}", pkg.arch);
    if let Some(url) = &pkg.url {
        println!("URL             : {url}");
    }
    println!("Licenses        : {}", VecDisp(&pkg.licenses));
    println!("Groups          : {}", VecDisp(&pkg.groups));
    println!("Provides        : {}", DepsDisp(&pkg.provides));
    println!("Depends On      : {}", DepsDisp(&pkg.depends));
    println!(
        "Optional Deps   : {}",
        OptDepsDisp {
            opt_deps: &pkg.opt_depends,
            pkgs,
            local: syncdb.is_none(),
        }
    );
    if syncdb.is_none() {
        println!(
            "Required By     : {}",
            RequiredByDisp::<false> { pkg, pkgs }
        );
        println!("Optional For    : {}", RequiredByDisp::<true> { pkg, pkgs });
    }
    println!("Conflicts With  : {}", VecDisp(&pkg.conflicts));
    println!("Replaces        : {}", VecDisp(&pkg.replaces));
    let mut sync_label = None;
    if syncdb.is_some() {
        let (size, label) = pacman_humanize_size(pkg.c_size, b'\0', -1);
        sync_label = Some(label);
        println!("Download Size   : {size:.2} {label}");
    }
    let (size, label) = pacman_humanize_size(
        pkg.size,
        sync_label.map_or(0, |label| label.as_bytes()[0]),
        -1,
    );
    let label = sync_label.unwrap_or(label);
    println!("Installed Size  : {size:.2} {label}");
    if let Some(packager) = &pkg.packager {
        println!("Packager        : {packager}");
    }
    let build_date = jiff::Timestamp::from_second(pkg.build_date.try_into().unwrap())
        .unwrap()
        .to_zoned(tz.clone());
    println!(
        "Build Date      : {}",
        build_date.strftime("%a %d %b %Y %I:%M:%S %p %Z")
    );
    if syncdb.is_none() {
        let install_date = jiff::Timestamp::from_second(pkg.install_date.try_into().unwrap())
            .unwrap()
            .to_zoned(tz.clone());
        println!(
            "Install Date    : {}",
            install_date.strftime("%a %d %b %Y %I:%M:%S %p %Z")
        );
        let reason = match pkg.install_reason {
            alpacka::InstallReason::Explicit => "Explicitly installed",
            alpacka::InstallReason::Dep => "Installed as a dependency for another package",
        };
        println!("Install Reason  : {reason}");
        let install_script = if pkg.install_script { "Yes" } else { "No" };
        println!("Install Script  : {}", install_script);
    }
    println!("Validated By    : {}", ValidationsDisp(&pkg.validations));
}

struct VecDisp<'a>(&'a [SmolStr]);

impl std::fmt::Display for VecDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            f.write_str("None")?;
        }
        for (i, item) in self.0.iter().enumerate() {
            let ws = if i == self.0.len() - 1 { "" } else { "  " };
            write!(f, "{item}{ws}")?;
        }
        Ok(())
    }
}

struct DepsDisp<'a>(&'a [Depend]);

impl std::fmt::Display for DepsDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            f.write_str("None")?;
        }
        for (i, dep) in self.0.iter().enumerate() {
            let ws = if i == self.0.len() - 1 { "" } else { "  " };
            let dep_str = match &dep.ver {
                Some(ver) => format!("{}{}{}", dep.name, ReqCmpDisp(&ver.req_cmp), ver.ver),
                None => dep.name.to_string(),
            };
            write!(f, "{dep_str}{ws}")?;
        }
        Ok(())
    }
}

struct OptDepsDisp<'a, 'b> {
    opt_deps: &'a [OptDepend],
    pkgs: &'b [Pkg],
    local: bool,
}

impl std::fmt::Display for OptDepsDisp<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.opt_deps.is_empty() {
            f.write_str("None")?;
        }
        for (i, opt_dep) in self.opt_deps.iter().enumerate() {
            let reason_str = match &opt_dep.reason {
                Some(reason) => format!(": {reason}"),
                None => String::new(),
            };
            if i != 0 {
                write!(f, "{}", " ".repeat(18))?;
            }
            let installed = self.pkgs.iter().any(|pkg| {
                pkg.desc.name == opt_dep.dep.name
                    || pkg
                        .desc
                        .provides
                        .iter()
                        .any(|provide| provide.name == opt_dep.dep.name)
            });
            let installed_str = if self.local && installed {
                " [installed]"
            } else {
                ""
            };
            let newline = if i == self.opt_deps.len() - 1 {
                ""
            } else {
                "\n"
            };
            let dep_str = match &opt_dep.dep.ver {
                Some(ver) => format!(
                    "{}{}{}",
                    opt_dep.dep.name,
                    ReqCmpDisp(&ver.req_cmp),
                    ver.ver
                ),
                None => opt_dep.dep.name.to_string(),
            };
            write!(f, "{dep_str}{reason_str}{installed_str}{newline}",)?;
        }
        Ok(())
    }
}

struct RequiredByDisp<'a, const OPT: bool> {
    pkg: &'a PkgDesc,
    pkgs: &'a [Pkg],
}

fn maybe_last_iter<I: Iterator>(iter: I) -> impl Iterator<Item = (I::Item, bool)> {
    let mut peekable = iter.peekable();
    std::iter::from_fn(move || {
        let item = peekable.next()?;
        let last = peekable.peek().is_none();
        Some((item, last))
    })
}

impl<const OPT: bool> std::fmt::Display for RequiredByDisp<'_, OPT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if OPT {
            required_by_disp_impl(f, self.pkg.optional_for(self.pkgs.iter()))?;
        } else {
            required_by_disp_impl(f, self.pkg.required_by(self.pkgs.iter()))?;
        };
        Ok(())
    }
}

fn required_by_disp_impl<'a, I: Iterator<Item = &'a Pkg>>(
    f: &mut std::fmt::Formatter<'_>,
    iter: I,
) -> std::fmt::Result {
    let mut any = false;
    for (pkg, last) in maybe_last_iter(iter) {
        any = true;
        if last {
            write!(f, "{}", pkg.desc.name)?;
        } else {
            write!(f, "{}  ", pkg.desc.name).unwrap();
        }
    }
    if !any {
        f.write_str("None")?;
    }
    Ok(())
}

struct ReqCmpDisp<'a>(&'a ReqCmp);

impl std::fmt::Display for ReqCmpDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self.0 {
            ReqCmp::Lt => "<",
            ReqCmp::LtEq => "<=",
            ReqCmp::Gt => ">",
            ReqCmp::GtEq => ">=",
            ReqCmp::Eq => "=",
        };
        f.write_str(s)
    }
}

struct ValidationsDisp<'a>(&'a [Validation]);

impl std::fmt::Display for ValidationsDisp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut any = false;
        for (validation, last) in maybe_last_iter(self.0.iter()) {
            any = true;
            let v_str = match validation {
                Validation::Pgp => "Signature",
                Validation::Sha256 => "SHA-256 Sum",
                Validation::Md5 => "MD5 Sum",
            };
            if last {
                write!(f, "{v_str}")?;
            } else {
                write!(f, "{v_str}  ")?;
            }
        }
        if !any {
            f.write_str("None")?;
        }
        Ok(())
    }
}
