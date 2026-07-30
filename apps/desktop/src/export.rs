use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use vox_proof::application_export::{
    render_application_decision_log, render_application_session_summary,
};
use vox_proof::application_export_v3::{
    render_application_decision_log_v3, render_application_session_summary_v3,
};
use vox_proof::application_service::ApplicationReviewExportBundle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPaths {
    pub reviewed_srt: PathBuf,
    pub decision_log: PathBuf,
    pub session_summary: PathBuf,
}

#[derive(Debug)]
pub enum ExportError {
    DestinationNotDirectory(PathBuf),
    Collision {
        paths: Vec<PathBuf>,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
        cleanup_failures: Vec<String>,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DestinationNotDirectory(path) => {
                write!(
                    formatter,
                    "destination is not a directory: {}",
                    path.display()
                )
            }
            Self::Collision { paths } => write!(
                formatter,
                "export refused because destination file(s) already exist: {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Write {
                path,
                source,
                cleanup_failures,
            } => {
                write!(formatter, "could not create {}: {source}", path.display())?;
                if !cleanup_failures.is_empty() {
                    write!(
                        formatter,
                        "; cleanup also failed: {}",
                        cleanup_failures.join("; ")
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ExportError {}

struct Payload {
    path: PathBuf,
    bytes: Vec<u8>,
}

trait ExclusiveIo {
    fn write_exclusive(&mut self, path: &Path, bytes: &[u8]) -> Result<(), ExclusiveWriteError>;
    fn remove_created(&mut self, path: &Path) -> std::io::Result<()>;
}

struct ExclusiveWriteError {
    source: std::io::Error,
    destination_was_created: bool,
}

struct FilesystemIo;

impl ExclusiveIo for FilesystemIo {
    fn write_exclusive(&mut self, path: &Path, bytes: &[u8]) -> Result<(), ExclusiveWriteError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source| ExclusiveWriteError {
                source,
                destination_was_created: false,
            })?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| ExclusiveWriteError {
                source,
                destination_was_created: true,
            })
    }

    fn remove_created(&mut self, path: &Path) -> std::io::Result<()> {
        fs::remove_file(path)
    }
}

pub fn export_bundle_v3_exclusively(
    base: &ApplicationReviewExportBundle,
    bundle_v3: &vox_proof::application_export_v3::ApplicationReviewExportBundleV3,
    destination: &Path,
    source_path: Option<&Path>,
) -> Result<ExportPaths, ExportError> {
    if !destination.is_dir() {
        return Err(ExportError::DestinationNotDirectory(
            destination.to_path_buf(),
        ));
    }

    let stem = source_path
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("transcript");
    let paths = ExportPaths {
        reviewed_srt: destination.join(format!("{stem}.voxproof-reviewed.srt")),
        decision_log: destination.join(format!("{stem}.voxproof-decisions-v3.txt")),
        session_summary: destination.join(format!("{stem}.voxproof-session-summary-v3.txt")),
    };
    let payloads = vec![
        Payload {
            path: paths.reviewed_srt.clone(),
            bytes: base.reviewed_srt.as_bytes().to_vec(),
        },
        Payload {
            path: paths.decision_log.clone(),
            bytes: render_application_decision_log_v3(bundle_v3).into_bytes(),
        },
        Payload {
            path: paths.session_summary.clone(),
            bytes: render_application_session_summary_v3(bundle_v3).into_bytes(),
        },
    ];

    let collisions = payloads
        .iter()
        .filter(|payload| payload.path.exists())
        .map(|payload| payload.path.clone())
        .collect::<Vec<_>>();
    if !collisions.is_empty() {
        return Err(ExportError::Collision { paths: collisions });
    }

    execute_payloads(&payloads, &mut FilesystemIo)?;
    Ok(paths)
}

pub fn export_bundle_exclusively(
    bundle: &ApplicationReviewExportBundle,
    destination: &Path,
    source_path: Option<&Path>,
) -> Result<ExportPaths, ExportError> {
    if !destination.is_dir() {
        return Err(ExportError::DestinationNotDirectory(
            destination.to_path_buf(),
        ));
    }

    let stem = source_path
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("transcript");
    let paths = ExportPaths {
        reviewed_srt: destination.join(format!("{stem}.voxproof-reviewed.srt")),
        decision_log: destination.join(format!("{stem}.voxproof-decisions.txt")),
        session_summary: destination.join(format!("{stem}.voxproof-session-summary.txt")),
    };
    let payloads = vec![
        Payload {
            path: paths.reviewed_srt.clone(),
            bytes: bundle.reviewed_srt.as_bytes().to_vec(),
        },
        Payload {
            path: paths.decision_log.clone(),
            bytes: render_application_decision_log(bundle).into_bytes(),
        },
        Payload {
            path: paths.session_summary.clone(),
            bytes: render_application_session_summary(bundle).into_bytes(),
        },
    ];

    let collisions = payloads
        .iter()
        .filter(|payload| payload.path.exists())
        .map(|payload| payload.path.clone())
        .collect::<Vec<_>>();
    if !collisions.is_empty() {
        return Err(ExportError::Collision { paths: collisions });
    }

    execute_payloads(&payloads, &mut FilesystemIo)?;
    Ok(paths)
}

fn execute_payloads(payloads: &[Payload], io: &mut impl ExclusiveIo) -> Result<(), ExportError> {
    let mut created: Vec<PathBuf> = Vec::new();
    for payload in payloads {
        if let Err(write_error) = io.write_exclusive(&payload.path, &payload.bytes) {
            if write_error.destination_was_created {
                created.push(payload.path.clone());
            }
            let cleanup_failures = created
                .iter()
                .rev()
                .filter_map(|path| {
                    io.remove_created(path)
                        .err()
                        .map(|error| format!("{}: {error}", path.display()))
                })
                .collect();
            return Err(ExportError::Write {
                path: payload.path.clone(),
                source: write_error.source,
                cleanup_failures,
            });
        }
        created.push(payload.path.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[derive(Default)]
    struct FailingIo {
        write_count: usize,
        removed: Vec<PathBuf>,
        cleanup_failure: bool,
        failed_destination_was_created: bool,
    }

    impl ExclusiveIo for FailingIo {
        fn write_exclusive(
            &mut self,
            _path: &Path,
            _bytes: &[u8],
        ) -> Result<(), ExclusiveWriteError> {
            self.write_count += 1;
            if self.write_count == 2 {
                Err(ExclusiveWriteError {
                    source: std::io::Error::other("injected second write failure"),
                    destination_was_created: self.failed_destination_was_created,
                })
            } else {
                Ok(())
            }
        }

        fn remove_created(&mut self, path: &Path) -> std::io::Result<()> {
            self.removed.push(path.to_path_buf());
            if self.cleanup_failure {
                Err(std::io::Error::other("injected cleanup failure"))
            } else {
                Ok(())
            }
        }
    }

    fn payloads() -> Vec<Payload> {
        ["one", "two", "three"]
            .into_iter()
            .map(|name| Payload {
                path: PathBuf::from(name),
                bytes: name.as_bytes().to_vec(),
            })
            .collect()
    }

    #[test]
    fn partial_failure_cleans_only_files_created_by_this_attempt() {
        let mut io = FailingIo::default();
        let error = execute_payloads(&payloads(), &mut io).unwrap_err();

        assert_eq!(io.removed, vec![PathBuf::from("one")]);
        assert!(matches!(
            error,
            ExportError::Write {
                path,
                cleanup_failures,
                ..
            } if path == Path::new("two") && cleanup_failures.is_empty()
        ));
    }

    #[test]
    fn original_write_error_and_cleanup_failure_are_both_reported() {
        let mut io = FailingIo {
            cleanup_failure: true,
            ..FailingIo::default()
        };
        let error = execute_payloads(&payloads(), &mut io).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("injected second write failure"));
        assert!(message.contains("injected cleanup failure"));
    }

    #[test]
    fn failed_destination_is_cleaned_when_creation_preceded_write_failure() {
        let mut io = FailingIo {
            failed_destination_was_created: true,
            ..FailingIo::default()
        };
        execute_payloads(&payloads(), &mut io).unwrap_err();
        assert_eq!(io.removed, vec![PathBuf::from("two"), PathBuf::from("one")]);
    }

    #[test]
    fn generated_paths_are_distinct() {
        let paths = ExportPaths {
            reviewed_srt: PathBuf::from("a"),
            decision_log: PathBuf::from("b"),
            session_summary: PathBuf::from("c"),
        };
        assert_eq!(
            HashSet::from([
                paths.reviewed_srt,
                paths.decision_log,
                paths.session_summary
            ])
            .len(),
            3
        );
    }
}
