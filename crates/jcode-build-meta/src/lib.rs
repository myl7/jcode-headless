//! Build and runtime version metadata for jcode.
//!
//! The build script (`build.rs`) computes git- and version-derived values and
//! emits them via `cargo:rustc-env`. All accessors below are thin wrappers over
//! those compile-time values.

/// Compile-time human-readable version string, e.g. `v0.14.6-dev (abc1234)`.
pub const VERSION: &str = env!("JCODE_VERSION");
/// Short git hash of the build commit, e.g. `abc1234` (or `unknown`).
pub const GIT_HASH: &str = env!("JCODE_GIT_HASH");
/// Commit date/time of the build commit (or `unknown`).
pub const GIT_DATE: &str = env!("JCODE_GIT_DATE");
/// `git describe --tags --always` output (may be empty).
pub const GIT_TAG: &str = env!("JCODE_GIT_TAG");
/// Compile-time auto-incrementing build semver (dev) or explicit release semver.
pub const SEMVER: &str = env!("JCODE_SEMVER");
/// Compile-time base semver taken from the root `Cargo.toml` package version.
pub const BASE_SEMVER: &str = env!("JCODE_BASE_SEMVER");
/// Compile-time root crate package version.
pub const PKG_VERSION: &str = env!("JCODE_PKG_VERSION");

/// Human-readable build version.
pub fn version() -> &'static str {
    VERSION
}

/// Git hash of the build commit.
pub fn git_hash() -> &'static str {
    GIT_HASH
}

/// Git date of the build commit.
pub fn git_date() -> &'static str {
    GIT_DATE
}

/// Git tag of the build commit.
pub fn git_tag() -> &'static str {
    GIT_TAG
}

/// Build semver.
pub fn semver() -> &'static str {
    SEMVER
}

/// Base semver from the root `Cargo.toml` package version.
pub fn base_semver() -> &'static str {
    BASE_SEMVER
}

/// Root crate package version.
pub fn pkg_version() -> &'static str {
    PKG_VERSION
}

/// Whether this binary was built as a release build (`JCODE_RELEASE_BUILD=1`).
pub fn is_release_build() -> bool {
    option_env!("JCODE_RELEASE_BUILD").is_some()
}
