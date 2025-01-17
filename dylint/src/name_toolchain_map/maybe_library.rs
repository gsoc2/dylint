use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaybeLibrary {
    inner: Inner,
}

impl MaybeLibrary {
    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }

    pub fn build(&self, opts: &crate::Dylint) -> Result<PathBuf> {
        self.inner.build(opts)
    }
}

impl From<PathBuf> for MaybeLibrary {
    fn from(path: PathBuf) -> Self {
        Self {
            inner: Inner::Path(path),
        }
    }
}

#[cfg(__metadata)]
impl From<crate::metadata::Package> for MaybeLibrary {
    fn from(package: crate::metadata::Package) -> Self {
        Self {
            inner: Inner::Package(package),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Inner {
    Path(PathBuf),

    #[cfg(__metadata)]
    Package(crate::metadata::Package),
}

impl Inner {
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Path(path) => path.clone(),

            #[cfg(__metadata)]
            Self::Package(package) => package.path(),
        }
    }

    #[cfg_attr(not(__metadata), allow(unused_variables))]
    fn build(&self, opts: &crate::Dylint) -> Result<PathBuf> {
        match self {
            Self::Path(path) => Ok(path.clone()),

            #[cfg(__metadata)]
            Self::Package(package) => crate::metadata::build_library(opts, package),
        }
    }
}
