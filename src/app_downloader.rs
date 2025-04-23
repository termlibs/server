use std::fmt::Display;
use rocket_okapi::JsonSchema;

fn get_extensions(filename: &str) -> Vec<String> {
    let parts: Vec<String> = filename.split('.').skip(1).map(|s| s.to_string()).collect();
    // this is lazy, need to fix this later (the len 4 part)
    parts.iter().filter(|x| !x.is_empty() && x.len() <= 4).cloned().collect()
}

#[derive(PartialEq, Debug)]
pub enum ArchiveType {
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    _7z,
    Zip,
    Rar,
    Gzip,
}

impl Display for ArchiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveType::Tar => write!(f, "tar"),
            ArchiveType::TarGz => write!(f, "tar.gz"),
            ArchiveType::TarBz2 => write!(f, "tar.bz2"),
            ArchiveType::TarXz => write!(f, "tar.xz"),
            ArchiveType::_7z => write!(f, "7z"),
            ArchiveType::Zip => write!(f, "zip"),
            ArchiveType::Rar => write!(f, "rar"),
            ArchiveType::Gzip => write!(f, "gz"),
        }
    }
}

impl ArchiveType {
    fn identify(input: &str) -> Option<ArchiveType> {
        let tar = ["tar"];
        let tar_gz = ["tar.gz", "tgz"];
        let tar_bz2 = ["tar.bz2"];
        let tar_xz = ["tar.xz"];
        let z7z = ["7z"];
        let rar = ["rar"];
        let gz = ["gz"];
        let zip = ["zip"];

        if tar.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::Tar);
        }
        if tar_gz.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::TarGz);
        }
        if tar_bz2.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::TarBz2);
        }
        if tar_xz.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::TarXz);
        }
        if z7z.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::_7z);
        }
        if zip.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::Zip);
        }
        if rar.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::Rar);
        }
        // should be last since it's sometimes a part of a compound
        if gz.iter().any(|x| input.ends_with(x)) {
            return Some(ArchiveType::Gzip);
        }
        None
    }
}

#[derive(PartialEq, Debug)]
pub enum InstallerType {
    Msi,
    Exe,
    Deb,
    Rpm,
    Pkg,
}

impl InstallerType {
    fn identify(input: &str) -> Option<InstallerType> {
        let extensions = &get_extensions(input);
        if extensions.is_empty() {
            return None;
        }
        match extensions.last().unwrap().as_str() {
            "msi" => Some(InstallerType::Msi),
            "exe" => Some(InstallerType::Exe),
            "deb" => Some(InstallerType::Deb),
            "rpm" => Some(InstallerType::Rpm),
            "pkg" => Some(InstallerType::Pkg),
            _ => None,
        }
    }
}

impl Display for InstallerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerType::Msi => write!(f, "msi"),
            InstallerType::Exe => write!(f, "exe"),
            InstallerType::Deb => write!(f, "deb"),
            InstallerType::Rpm => write!(f, "rpm"),
            InstallerType::Pkg => write!(f, "pkg"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Filetype {
    Binary,
    Installer(InstallerType),
    Archive(ArchiveType),
    Unknown,
}

impl Display for Filetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filetype::Binary => write!(f, "binary"),
            Filetype::Archive(x) => write!(f, "{}", x),
            Filetype::Installer(x) => write!(f, "{:?} installer", x),
            Filetype::Unknown => write!(f, "unknown"),
        }
    }
}

impl Filetype {
    fn identify(input: &str) -> Filetype {
        let archive = ArchiveType::identify(input);
        if archive.is_some() {
            return Filetype::Archive(archive.unwrap());
        }
        let installer_type = InstallerType::identify(input);
        if installer_type.is_some() {
            return Filetype::Installer(installer_type.unwrap());
        }

        let extensions = get_extensions(input);
        if extensions.is_empty() || {
            match extensions.last().unwrap_or(&String::default()).as_str() {
                "exe" => true,
                _ => false,
            }
        } {
            Filetype::Binary
        } else {
            Filetype::Unknown
        }
    }
}

#[derive(PartialEq, Debug, FromFormField,JsonSchema)]
pub enum TargetOs {
    Windows,
    Linux,
    Mac,
    Freebsd,
    Openbsd,
    Netbsd,
    Unknown,
}

impl From<&str> for TargetOs {
    fn from(input: &str) -> Self {
        TargetOs::identify(input)
    }
}

impl TargetOs {
    fn identify(input: &str) -> TargetOs {
        let normed_input = input.to_lowercase();
        let win = ["win", "windows"];
        let linux = ["linux"];
        let mac = ["mac", "macos", "macosx", "darwin"];
        let freebsd = ["freebsd"];
        let openbsd = ["openbsd"];
        let netbsd = ["netbsd"];

        // bsd's first since there really isn't a false positive here
        if freebsd.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Freebsd;
        }
        if openbsd.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Openbsd;
        }
        if netbsd.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Netbsd;
        }
        // note, mac must go before win since "win" is in "darwin"
        if mac.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Mac;
        }
        if win.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Windows;
        }
        if linux.iter().any(|x| normed_input.contains(x)) {
            return TargetOs::Linux;
        }
        TargetOs::Unknown
    }
}

impl Display for TargetOs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetOs::Windows => write!(f, "windows"),
            TargetOs::Linux => write!(f, "linux"),
            TargetOs::Mac => write!(f, "mac"),
            TargetOs::Freebsd => write!(f, "freebsd"),
            TargetOs::Openbsd => write!(f, "openbsd"),
            TargetOs::Netbsd => write!(f, "netbsd"),
            TargetOs::Unknown => write!(f, "unknown"),
        }
    }
}


#[derive(PartialEq, Debug, FromFormField,JsonSchema)]
pub(crate) enum TargetArch {
    Amd64,
    Arm64,
    Aarch64,
    PPCLe,
    PPC,
    Arm32,
    MipsLe,
    Mips,
    Mips64Le,
    Mips64,
    RiscV,
    #[allow(non_camel_case_types)]
    x86,
    Unknown,
}

impl From<&str> for TargetArch {
    fn from(value: &str) -> Self {
        TargetArch::identify(value)
    }
}

impl Display for TargetArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetArch::Amd64 => write!(f, "amd64"),
            TargetArch::Arm64 => write!(f, "arm64"),
            TargetArch::Aarch64 => write!(f, "aarch64"),
            TargetArch::PPC => write!(f, "ppc64"),
            TargetArch::PPCLe => write!(f, "ppc64le"),
            TargetArch::Arm32 => write!(f, "arm"),
            TargetArch::Mips => write!(f, "mips"),
            TargetArch::MipsLe => write!(f, "mipsle"),
            TargetArch::Mips64 => write!(f, "mips64"),
            TargetArch::Mips64Le => write!(f, "mips64le"),
            TargetArch::RiscV => write!(f, "riscv"),
            TargetArch::x86 => write!(f, "x86"),
            TargetArch::Unknown => write!(f, "unknown"),
        }
    }
}

impl TargetArch {
    fn identify(input: &str) -> TargetArch {
        let amd = ["amd64", "x64", "x86_64"];
        let x86 = ["x86", "i386", "i686", "x86_32", "386", "686", "ia32"];
        let arm = ["arm64"];
        let arm32 = ["arm"];
        let aarch = ["aarch64"];
        let ppcle = ["ppc64le", "ppcle"];
        let ppc = ["ppc", "ppc64"];
        let mips64le = ["mips64le"];
        let mips64 = ["mips64"];
        let mipsle = ["mipsle"];
        let mips = ["mips"];
        let riscv = ["riscv"];

        // order of these gates is important due to the substrings

        if amd.iter().any(|x| input.contains(x)) {
            return TargetArch::Amd64;
        }
        if arm.iter().any(|x| input.contains(x)) {
            return TargetArch::Arm64;
        }
        if aarch.iter().any(|x| input.contains(x)) {
            return TargetArch::Aarch64;
        }
        if ppcle.iter().any(|x| input.contains(x)) {
            return TargetArch::PPCLe;
        }
        if ppc.iter().any(|x| input.contains(x)) {
            return TargetArch::PPC;
        }
        if mips64le.iter().any(|x| input.contains(x)) {
            return TargetArch::Mips64Le;
        }
        if mips64.iter().any(|x| input.contains(x)) {
            return TargetArch::Mips64;
        }
        if mipsle.iter().any(|x| input.contains(x)) {
            return TargetArch::MipsLe;
        }
        if mips.iter().any(|x| input.contains(x)) {
            return TargetArch::Mips;
        }
        if x86.iter().any(|x| input.contains(x)) {
            return TargetArch::x86;
        }
        if arm32.iter().any(|x| input.contains(x)) {
            return TargetArch::Arm32;
        }
        if riscv.iter().any(|x| input.contains(x)) {
            return TargetArch::RiscV;
        }

        // ok, some that come from combining with the os name
        let amd64_os = ["windows64", "win64", "winx64", "linux64"];
        let x86_os = ["windows32", "win32", "winx86", "linux32"];
        if amd64_os.iter().any(|x| input.contains(x)) {
            return TargetArch::Amd64;
        }
        if x86_os.iter().any(|x| input.contains(x)) {
            return TargetArch::x86;
        }

        TargetArch::Unknown
    }
}

#[derive(PartialEq, Debug)]
pub struct TargetDeployment {
    os: TargetOs,
    arch: TargetArch,
}

impl Display for TargetDeployment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

impl TargetDeployment {
    fn is_unknown(&self) -> bool {
        self.os == TargetOs::Unknown || self.arch == TargetArch::Unknown
    }

    fn identify(input: &str) -> TargetDeployment {
        TargetDeployment {
            os: TargetOs::identify(input),
            arch: TargetArch::identify(input),
        }
    }
}

impl Default for TargetDeployment {
    fn default() -> Self {
        TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Amd64,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Target {
    pub deployment: TargetDeployment,
    pub filetype: Filetype,
}

impl Target {
    pub(crate) fn identify(input: &str) -> Target {
        Target {
            deployment: TargetDeployment::identify(input),
            filetype: Filetype::identify(input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ITC {
        input: String,
        expected: Target,
    }

    impl ITC {
        fn new(input: &str, expected: Target) -> ITC {
            ITC {
                input: input.to_string(),
                expected,
            }
        }
    }

    #[test]
    fn test_assumptions() {}

    #[test]
    fn test_identify() {
        let cases = vec![
            ITC::new(
                "yq_darwin_amd64",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Mac,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_darwin_amd64.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Mac,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_darwin_arm64",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Mac,
                        arch: TargetArch::Arm64,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_darwin_arm64.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Mac,
                        arch: TargetArch::Arm64,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_freebsd_386",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::x86,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_freebsd_386.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::x86,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_freebsd_amd64",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_freebsd_amd64.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_freebsd_arm",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::Arm32,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_freebsd_arm.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Freebsd,
                        arch: TargetArch::Arm32,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_linux_386",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::x86,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_386.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::x86,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_linux_amd64",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_amd64.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Amd64,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_linux_arm",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Arm32,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_mips",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_mips.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_linux_mips64",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips64,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_mips64.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips64,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
            ITC::new(
                "yq_linux_mips64le",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips64Le,
                    },
                    filetype: Filetype::Binary,
                },
            ),
            ITC::new(
                "yq_linux_mips64le.tar.gz",
                Target {
                    deployment: TargetDeployment {
                        os: TargetOs::Linux,
                        arch: TargetArch::Mips64Le,
                    },
                    filetype: Filetype::Archive(ArchiveType::TarGz),
                },
            ),
        ];

        for case in cases {
            println!("{:?} -> {:?}", case.input, case.expected);
            assert_eq!(Target::identify(&case.input), case.expected);
        }
    }
}
