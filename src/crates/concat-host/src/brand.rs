//! What the product is called and where it lives: the one place to change
//! when the name, the repository or the website moves.
//!
//! Slint files cannot read these, so the window title and the version line
//! in `concat/ui` repeat [`NAME`]; `scripts/check-brand.sh` fails if they
//! drift.
//!
//! CutZone is a rebranded fork of Concat (AGPL-3.0-or-later). The crate
//! names (`concat-*`) are kept on purpose so upstream fixes still merge.

/// The product's name as a person reads it: one word, capital C and Z.
pub const NAME: &str = "CutZone";

/// The folder new projects are made in on the desktop and the phone.
pub const PROJECTS_FOLDER: &str = "CutZone";

/// The repository the code, the issues and the releases live in.
pub const REPOSITORY: &str = "https://github.com/mr-socialmedia/concat";

/// The same repository as `owner/name`, which is what the updater reads the
/// releases of. Pointing this at anything else makes the app offer another
/// product's installer as its own update.
pub const REPO: &str = "mr-socialmedia/concat";

/// GitHub's list of that repository's releases, newest first.
pub const RELEASES_API: &str =
    "https://api.github.com/repos/mr-socialmedia/concat/releases?per_page=100";

/// Where a person goes to read how to use it, until there is a website.
pub const DOCS: &str = "https://github.com/mr-socialmedia/concat/tree/HEAD/docs";

/// Where a person goes to report a problem.
pub const REPORT: &str = "https://github.com/mr-socialmedia/concat/issues/new";

#[cfg(test)]
mod tests {
    use super::*;

    /// The three spellings of the repository agree, so the updater, the
    /// links and the release list cannot point at different places.
    #[test]
    fn the_repository_is_spelled_the_same_everywhere() {
        assert_eq!(REPOSITORY, format!("https://github.com/{REPO}"));
        assert_eq!(
            RELEASES_API,
            format!("https://api.github.com/repos/{REPO}/releases?per_page=100")
        );
        assert!(DOCS.starts_with(REPOSITORY) && REPORT.starts_with(REPOSITORY));
    }

    /// The updater must never read another product's releases: pointing it
    /// at upstream Concat would offer its installer as a CutZone update.
    #[test]
    fn the_updater_never_reads_upstream_concat() {
        for url in [REPO, REPOSITORY, RELEASES_API, DOCS, REPORT] {
            assert!(!url.to_lowercase().contains("jub0t"), "{url}");
        }
    }

    /// The name is one word with a capital C and Z.
    #[test]
    fn the_name_is_one_word() {
        assert_eq!(NAME, "CutZone");
        assert!(!NAME.contains(' '));
    }
}
