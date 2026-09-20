//! Canonical project resolution.
//!
//! Every command that scopes work to a project goes through [`resolve_project`]
//! so that "the project" means one thing everywhere: a canonical directory,
//! the `conscience.yaml` found there, and any worktrees the manifest declares
//! as the same project.
//!
//! Claude Code stores each project's sessions under
//! `~/.claude/projects/<encoded path>`, where the encoding replaces every
//! non-alphanumeric character with `-`. That encoding is lossy
//! (`general.legal` and `general-legal` collide), so matching always goes
//! from a real path to its encoded form, never the other way round.

use crate::error::{ConscienceError, Result};
use crate::ethics::manifest::Manifest;
use std::path::{Path, PathBuf};

/// A resolved project: the canonical root plus any worktree aliases.
#[derive(Debug, Clone)]
pub struct ProjectScope {
    /// Canonical path of the project root (symlinks resolved).
    pub root: PathBuf,
    /// Canonical paths of additional checkouts that count as this project.
    pub worktrees: Vec<PathBuf>,
    /// The manifest loaded from `root`, if one exists.
    pub manifest: Option<Manifest>,
    /// Whether the root was given explicitly (`--project`) or defaulted to cwd.
    pub explicit: bool,
    /// `owner/repo` from the checkout's `origin` remote, when it is on GitHub.
    pub remote_repo: Option<String>,
}

/// Where a GitHub repository name came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoSource {
    /// `--repo` on the command line.
    Flag,
    /// `github.repo` in conscience.yaml.
    Manifest,
    /// The checkout's `origin` remote.
    Remote,
}

impl std::fmt::Display for RepoSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RepoSource::Flag => "--repo",
            RepoSource::Manifest => "conscience.yaml",
            RepoSource::Remote => "origin remote",
        })
    }
}

impl ProjectScope {
    /// Build a scope from an already-canonical root with no manifest lookup.
    /// Used by tests and by callers that construct scopes programmatically.
    pub fn from_root(root: PathBuf) -> Self {
        Self {
            root,
            worktrees: Vec::new(),
            manifest: None,
            explicit: true,
            remote_repo: None,
        }
    }

    /// The GitHub repository to use, in priority order: an explicit flag,
    /// then `github.repo` in the manifest, then the `origin` remote.
    pub fn github_repo(&self, explicit: Option<&str>) -> Option<(String, RepoSource)> {
        if let Some(r) = explicit.filter(|s| !s.trim().is_empty()) {
            return Some((r.trim().to_string(), RepoSource::Flag));
        }
        if let Some(r) = self
            .manifest
            .as_ref()
            .and_then(|m| m.github.repo.clone())
            .filter(|s| !s.trim().is_empty())
        {
            return Some((r.trim().to_string(), RepoSource::Manifest));
        }
        self.remote_repo
            .clone()
            .map(|r| (r, RepoSource::Remote))
    }

    /// All directories that count as this project: root first, then worktrees.
    pub fn roots(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.root).chain(self.worktrees.iter())
    }

    /// The Claude Code directory names this project's sessions live under.
    pub fn encoded_dirs(&self) -> Vec<String> {
        self.roots().map(|p| encode_project_dir(p)).collect()
    }

    /// Does a Claude Code project directory name belong to this project?
    pub fn matches_encoded_dir(&self, dir_name: &str) -> bool {
        self.roots().any(|p| encode_project_dir(p) == dir_name)
    }

    /// Does a session's recorded working directory belong to this project?
    ///
    /// Canonicalizes `cwd` when it still exists so symlinked checkouts match;
    /// falls back to a string comparison when it does not.
    pub fn matches_cwd(&self, cwd: &str) -> bool {
        let candidate = Path::new(cwd);
        let canonical = candidate.canonicalize().ok();
        self.roots()
            .any(|root| canonical.as_deref() == Some(root.as_path()) || candidate == root.as_path())
    }

    /// Display name: manifest name if set, else the root directory name.
    pub fn display_name(&self) -> String {
        self.manifest
            .as_ref()
            .map(|m| m.project.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| {
                self.root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.root.display().to_string())
            })
    }
}

/// Resolve the project a command should operate on.
///
/// `explicit` is the `--project` argument if given; otherwise the current
/// directory is used. The path must exist. Worktree aliases come from
/// `project.worktrees` in the manifest; ones that do not exist on disk are
/// skipped with a warning rather than failing the command.
pub fn resolve_project(explicit: Option<&Path>) -> Result<ProjectScope> {
    let (requested, was_explicit) = match explicit {
        Some(p) => (p.to_path_buf(), true),
        None => (
            std::env::current_dir().map_err(|e| {
                ConscienceError::Other(anyhow::anyhow!("Cannot determine current directory: {}", e))
            })?,
            false,
        ),
    };

    let root = requested.canonicalize().map_err(|_| {
        ConscienceError::Other(anyhow::anyhow!(
            "Project path does not exist: {}",
            requested.display()
        ))
    })?;

    let manifest = Manifest::load(&root);

    let mut worktrees = Vec::new();
    if let Some(m) = &manifest {
        for declared in &m.project.worktrees {
            let candidate = expand_home(declared);
            let candidate = if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
            match candidate.canonicalize() {
                Ok(c) if c != root && !worktrees.contains(&c) => worktrees.push(c),
                Ok(_) => {}
                Err(_) => eprintln!(
                    "Warning: worktree '{}' in conscience.yaml does not exist; skipping",
                    declared
                ),
            }
        }
    }

    let remote_repo = crate::github::remote::detect_origin_repo(&root);

    Ok(ProjectScope {
        root,
        worktrees,
        manifest,
        explicit: was_explicit,
        remote_repo,
    })
}

/// Encode a path the way Claude Code names its per-project directories:
/// every character that is not ASCII alphanumeric becomes `-`.
pub fn encode_project_dir(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn expand_home(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_slashes_dots_and_dashes_identically() {
        // Real example from a user's ~/.claude/projects: a dotted directory
        // and a dashed one both flatten to the same shape.
        assert_eq!(
            encode_project_dir(Path::new(
                "/Users/mkonda/MKWork/kondasecurity/general.legal"
            )),
            "-Users-mkonda-MKWork-kondasecurity-general-legal"
        );
        assert_eq!(
            encode_project_dir(Path::new(
                "/Users/mkonda/MKWork/kondasecurity/general-legal"
            )),
            "-Users-mkonda-MKWork-kondasecurity-general-legal"
        );
        assert_eq!(
            encode_project_dir(Path::new("/home/dev/my_repo")),
            "-home-dev-my-repo"
        );
    }

    #[test]
    fn matches_only_exact_encoded_dir() {
        let scope = ProjectScope::from_root(PathBuf::from("/Users/dev/work/conscience"));
        assert!(scope.matches_encoded_dir("-Users-dev-work-conscience"));
        // A containing path must not match: the old containment bug.
        assert!(!scope.matches_encoded_dir("-Users-dev-work-conscience-dashboard"));
        assert!(!scope.matches_encoded_dir("-Users-dev-work"));
    }

    #[test]
    fn worktrees_match_as_the_same_project() {
        let mut scope = ProjectScope::from_root(PathBuf::from("/Users/dev/work/conscience"));
        scope
            .worktrees
            .push(PathBuf::from("/Users/dev/work/conscience-wt-feature"));
        assert!(scope.matches_encoded_dir("-Users-dev-work-conscience-wt-feature"));
        assert!(scope.matches_cwd("/Users/dev/work/conscience-wt-feature"));
        assert!(!scope.matches_cwd("/Users/dev/work/other"));
    }

    #[test]
    fn github_repo_prefers_flag_then_manifest_then_remote() {
        let mut scope = ProjectScope::from_root(PathBuf::from("/work/app"));
        assert_eq!(scope.github_repo(None), None);

        scope.remote_repo = Some("acme/app".into());
        assert_eq!(
            scope.github_repo(None),
            Some(("acme/app".into(), RepoSource::Remote))
        );

        let mut m = Manifest::default();
        m.github.repo = Some("acme/app-upstream".into());
        scope.manifest = Some(m);
        assert_eq!(
            scope.github_repo(None),
            Some(("acme/app-upstream".into(), RepoSource::Manifest))
        );

        assert_eq!(
            scope.github_repo(Some("other/thing")),
            Some(("other/thing".into(), RepoSource::Flag))
        );
        // Blank flag and blank manifest values do not count.
        assert_eq!(
            scope.github_repo(Some("  ")),
            Some(("acme/app-upstream".into(), RepoSource::Manifest))
        );
    }

    #[test]
    fn resolve_detects_origin_remote_of_a_git_checkout() {
        let tmp = std::env::temp_dir().join(format!("conscience-remote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(&tmp)
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            assert!(ok, "git {:?}", args);
        };
        git(&["init", "-q"]);
        git(&["remote", "add", "origin", "git@github.com:acme/detected.git"]);

        let scope = resolve_project(Some(&tmp)).unwrap();
        assert_eq!(scope.remote_repo.as_deref(), Some("acme/detected"));
        assert_eq!(
            scope.github_repo(None),
            Some(("acme/detected".into(), RepoSource::Remote))
        );

        // A non-GitHub remote is simply not a GitHub repo.
        git(&["remote", "set-url", "origin", "git@gitlab.com:acme/detected.git"]);
        let scope = resolve_project(Some(&tmp)).unwrap();
        assert_eq!(scope.remote_repo, None);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn resolves_explicit_directory_canonically() {
        let tmp = std::env::temp_dir().join(format!("conscience-proj-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let dotted = tmp.join("sub").join("..").join("sub");
        std::fs::create_dir_all(tmp.join("sub")).unwrap();

        let scope = resolve_project(Some(&dotted)).expect("resolves");
        assert_eq!(scope.root, tmp.join("sub").canonicalize().unwrap());
        assert!(scope.explicit);
        assert!(scope.manifest.is_none());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn missing_directory_is_an_error() {
        let err = resolve_project(Some(Path::new("/definitely/not/a/real/path/xyz")))
            .err()
            .expect("should fail");
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn manifest_worktrees_are_loaded_and_missing_ones_skipped() {
        let tmp = std::env::temp_dir().join(format!("conscience-wt-{}", std::process::id()));
        let root = tmp.join("main");
        let wt = tmp.join("feature");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(
            root.join("conscience.yaml"),
            format!(
                "project:\n  name: Demo\n  worktrees:\n    - {}\n    - {}\n",
                wt.display(),
                tmp.join("nope").display()
            ),
        )
        .unwrap();

        let scope = resolve_project(Some(&root)).expect("resolves");
        assert_eq!(scope.worktrees, vec![wt.canonicalize().unwrap()]);
        assert_eq!(scope.display_name(), "Demo");

        std::fs::remove_dir_all(&tmp).ok();
    }
}
