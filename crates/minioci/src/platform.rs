use oci_spec::image::{Arch, Os};

#[derive(Clone, Debug)]
pub struct OciPlatform {
    pub os: Os,
    pub arch: Arch,
}

impl OciPlatform {
    #[cfg(target_arch = "x86_64")]
    const CURRENT_ARCH: Arch = Arch::Amd64;
    #[cfg(target_arch = "aarch64")]
    const CURRENT_ARCH: Arch = Arch::ARM64;

    pub fn new(os: Os, arch: Arch) -> OciPlatform {
        OciPlatform { os, arch }
    }

    pub fn current() -> OciPlatform {
        OciPlatform {
            os: Os::Linux,
            arch: OciPlatform::CURRENT_ARCH,
        }
    }
}
