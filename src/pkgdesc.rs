use smol_str::SmolStr;

#[derive(Debug)]
pub struct PkgDesc {
    pub name: SmolStr,
    pub version: SmolStr,
    pub desc: Option<SmolStr>,
    pub arch: SmolStr,
    pub url: Option<SmolStr>,
    pub licenses: Vec<SmolStr>,
    pub depends: Vec<Depend>,
    pub opt_depends: Vec<OptDepend>,
    pub provides: Vec<Depend>,
    pub conflicts: Vec<SmolStr>,
    pub replaces: Vec<SmolStr>,
    pub size: u64,
    /// Compressed size
    pub c_size: u64,
    pub packager: Option<SmolStr>,
    pub groups: Vec<SmolStr>,
    pub build_date: u64,
    pub install_date: u64,
    pub install_reason: InstallReason,
    pub install_script: bool,
    pub validations: Vec<Validation>,
}

#[derive(Debug)]
pub struct Depend {
    pub name: SmolStr,
    pub ver: Option<DepVer>,
}

#[derive(Debug)]
pub struct DepVer {
    pub req_cmp: ReqCmp,
    pub ver: SmolStr,
}

impl DepVer {
    fn satisfies(&self, other: &Self) -> bool {
        match other.req_cmp {
            ReqCmp::Lt => self.ver < other.ver,
            ReqCmp::Gt => self.ver > other.ver,
            ReqCmp::Eq => self.ver == other.ver,
            ReqCmp::LtEq => self.ver <= other.ver,
            ReqCmp::GtEq => self.ver >= other.ver,
        }
    }
}

#[derive(Debug)]
pub enum ReqCmp {
    Lt,
    LtEq,
    Gt,
    GtEq,
    Eq,
}

impl Depend {
    fn parse(src: &str) -> Self {
        match src.find(['=', '>', '<']) {
            Some(pos) => {
                let name = &src[..pos];
                let ver_cmp_part = &src[pos..];
                let mut ver_offs = 1;
                let req_cmp = if ver_cmp_part.starts_with("<=") {
                    ver_offs = 2;
                    ReqCmp::LtEq
                } else if ver_cmp_part.starts_with('<') {
                    ReqCmp::Lt
                } else if ver_cmp_part.starts_with(">=") {
                    ver_offs = 2;
                    ReqCmp::GtEq
                } else if ver_cmp_part.starts_with('>') {
                    ReqCmp::Gt
                } else if ver_cmp_part.starts_with('=') {
                    ReqCmp::Eq
                } else {
                    panic!("Unknown req ({ver_cmp_part})");
                };
                let ver = &ver_cmp_part[ver_offs..];
                Self {
                    name: name.into(),
                    ver: Some(DepVer {
                        req_cmp,
                        ver: ver.into(),
                    }),
                }
            }
            None => Self {
                name: src.into(),
                ver: None,
            },
        }
    }
    /// Checks if this dependency satisfies a requirement dependency
    ///
    /// # Panics
    ///
    /// If the requirement has a version, but this dependnecy doesn't
    #[must_use]
    pub fn satisfies(&self, req: &Self) -> bool {
        if self.name == req.name {
            match &req.ver {
                Some(ver) => self.ver.as_ref().unwrap().satisfies(ver),
                None => true,
            }
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub enum InstallReason {
    Explicit,
    Dep,
}

#[derive(Debug)]
pub enum Validation {
    Pgp,
    Sha256,
    Md5,
}

#[derive(Debug)]
pub struct OptDepend {
    pub dep: Depend,
    pub reason: Option<SmolStr>,
}

impl PkgDesc {
    /// Parse a package description file
    ///
    /// # Panics
    ///
    /// Panics on missing mandatory keys, like `name` and `version`.
    #[must_use]
    pub fn parse(src: &str, install_script: bool) -> Self {
        let mut section = None;
        let [
            mut name,
            mut version,
            mut arch,
            mut desc,
            mut url,
            mut packager,
        ] = [const { None }; 6];
        let mut depends = Vec::new();
        let mut opt_depends = Vec::new();
        let mut licenses = Vec::new();
        let mut provides = Vec::new();
        let mut conflicts = Vec::new();
        let mut replaces = Vec::new();
        let mut groups = Vec::new();
        let mut size = 0;
        let mut c_size = 0;
        let mut build_date = 0;
        let mut install_date = 0;
        let mut install_reason = InstallReason::Explicit;
        let mut validations = Vec::new();
        for line in src.lines() {
            if line.is_empty() {
                section = None;
                continue;
            }
            match section {
                Some(section) => match section {
                    "NAME" => name = Some(line.into()),
                    "VERSION" => version = Some(line.into()),
                    "ARCH" => arch = Some(line.into()),
                    "DESC" => desc = Some(line.into()),
                    "URL" => url = Some(line.into()),
                    "DEPENDS" => depends.push(Depend::parse(line)),
                    "OPTDEPENDS" => {
                        let (dep, reason) = match line.split_once(": ") {
                            Some((pkg, reason)) => (pkg, Some(reason.into())),
                            None => (line, None),
                        };
                        opt_depends.push(OptDepend {
                            dep: Depend::parse(dep),
                            reason,
                        });
                    }
                    "LICENSE" => licenses.push(line.into()),
                    "PROVIDES" => provides.push(Depend::parse(line)),
                    "CONFLICTS" => conflicts.push(line.into()),
                    "REPLACES" => replaces.push(line.into()),
                    "GROUPS" => groups.push(line.into()),
                    "SIZE" | "ISIZE" => size = line.parse().unwrap(),
                    "CSIZE" => c_size = line.parse().unwrap(),
                    "PACKAGER" => packager = Some(line.into()),
                    "BUILDDATE" => build_date = line.parse().unwrap(),
                    "INSTALLDATE" => install_date = line.parse().unwrap(),
                    "REASON" => {
                        install_reason = match line {
                            "0" => InstallReason::Explicit,
                            _ => InstallReason::Dep,
                        }
                    }
                    #[expect(clippy::single_match)]
                    "VALIDATION" => match line {
                        "pgp" => validations.push(Validation::Pgp),
                        _ => {}
                    },
                    "SHA256SUM" => validations.push(Validation::Sha256),
                    "MD5SUM" => validations.push(Validation::Md5),
                    "PGPSIG" => validations.push(Validation::Pgp),
                    _ => {}
                },
                None => {
                    section = Some(&line[1..line.len() - 1]);
                }
            }
        }
        Self {
            name: name.unwrap(),
            version: version.unwrap(),
            arch: arch.unwrap(),
            desc,
            licenses,
            url,
            depends,
            opt_depends,
            provides,
            conflicts,
            replaces,
            size,
            c_size,
            packager,
            groups,
            build_date,
            install_date,
            install_reason,
            install_script,
            validations,
        }
    }
}
