use crate::UpdateError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    WindowsX64,
    MacosArm64,
    MacosX64,
    LinuxX64,
}

impl Platform {
    pub const ALL: [Self; 4] = [Self::WindowsX64, Self::MacosArm64, Self::MacosX64, Self::LinuxX64];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WindowsX64 => "windows-x64",
            Self::MacosArm64 => "macos-aarch64",
            Self::MacosX64 => "macos-x86_64",
            Self::LinuxX64 => "linux-x64",
        }
    }
    pub fn channel(self) -> String {
        format!("stable-{}", self.as_str())
    }
    pub fn current() -> Option<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("windows", "x86_64") => Some(Self::WindowsX64),
            ("macos", "aarch64") => Some(Self::MacosArm64),
            ("macos", "x86_64") => Some(Self::MacosX64),
            ("linux", "x86_64") => Some(Self::LinuxX64),
            _ => None,
        }
    }
}
impl std::str::FromStr for Platform {
    type Err = UpdateError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str() == value)
            .ok_or(UpdateError::Platform)
    }
}
