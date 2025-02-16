use crate::{Pkg, PkgDesc};

#[must_use]
pub fn pkg_matches_dep(pkg: &PkgDesc, dependent_pkg: &PkgDesc) -> bool {
    dependent_pkg
        .depends
        .iter()
        .any(|dep| dep.name == pkg.name || pkg.provides.iter().any(|prov| prov.satisfies(dep)))
}

#[must_use]
pub fn pkg_matches_opt_dep(pkg: &PkgDesc, dependent_pkg: &PkgDesc) -> bool {
    dependent_pkg.opt_depends.iter().any(|opt_dep| {
        opt_dep.dep.name == pkg.name
            || pkg
                .provides
                .iter()
                .any(|prov| prov.name == opt_dep.dep.name)
    })
}

fn pkg_deps<'pkgs, const OPT: bool>(
    pkg: &PkgDesc,
    pkgs: impl Iterator<Item = &'pkgs Pkg>,
) -> impl Iterator<Item = &'pkgs Pkg> {
    pkgs.filter(|dependent_pkg| {
        if OPT {
            pkg_matches_opt_dep(pkg, &dependent_pkg.desc)
        } else {
            pkg_matches_dep(pkg, &dependent_pkg.desc)
        }
    })
}

pub trait PkgDepsExt {
    fn required_by<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg>;
    fn optional_for<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg>;
}

impl PkgDepsExt for Pkg {
    fn required_by<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg> {
        pkg_deps::<false>(&self.desc, pkgs)
    }

    fn optional_for<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg> {
        pkg_deps::<true>(&self.desc, pkgs)
    }
}

impl PkgDepsExt for PkgDesc {
    fn required_by<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg> {
        pkg_deps::<false>(self, pkgs)
    }

    fn optional_for<'pkgs>(
        &self,
        pkgs: impl Iterator<Item = &'pkgs Pkg>,
    ) -> impl Iterator<Item = &'pkgs Pkg> {
        pkg_deps::<true>(self, pkgs)
    }
}
