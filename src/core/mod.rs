//! 앱 전역 설정 + 인스턴스(프로필) 정의

pub mod config;
pub mod instance;

pub use config::LauncherConfig;
pub use instance::{InstalledMod, Instance, LoaderType, ModSource};
