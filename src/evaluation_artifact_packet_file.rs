use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use crate::evaluation_artifact_packet::{
    EncodedEvaluationArtifactPacket, EvaluationArtifactPacketDigest, EvaluationArtifactPacketError,
    VerifiedEvaluationArtifactPacket, decode_and_verify_packet,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationArtifactPacketFileLimits {
    pub max_packet_bytes: u64,
}

impl EvaluationArtifactPacketFileLimits {
    pub fn new(max_packet_bytes: u64) -> Result<Self, EvaluationArtifactPacketFileError> {
        if max_packet_bytes == 0 {
            return Err(EvaluationArtifactPacketFileError::InvalidFileLimit);
        }
        if max_packet_bytes > usize::MAX as u64 {
            return Err(EvaluationArtifactPacketFileError::InvalidFileLimit);
        }
        Ok(Self { max_packet_bytes })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationArtifactPacketFileWriteReceipt {
    pub content_digest: EvaluationArtifactPacketDigest,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEvaluationArtifactPacketFile {
    pub verified_packet: VerifiedEvaluationArtifactPacket,
    pub content_digest: EvaluationArtifactPacketDigest,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationArtifactPacketFileError {
    InvalidFileLimit,
    EncodedPacketLengthMismatch,
    EncodedPacketExceedsLimit,
    EncodedPacketVerificationFailure(EvaluationArtifactPacketError),
    DestinationAlreadyExists,
    UnsupportedFileType,
    FileMetadataFailure { kind: io::ErrorKind },
    FileOpenFailure { kind: io::ErrorKind },
    FileCreateFailure { kind: io::ErrorKind },
    FileReadFailure { kind: io::ErrorKind },
    FileWriteFailure { kind: io::ErrorKind },
    FileFlushFailure { kind: io::ErrorKind },
    FileSyncFailure { kind: io::ErrorKind },
    FileExceedsLimit,
    FileLengthConversionOverflow,
    PostWriteByteMismatch,
    PostWritePacketVerificationFailure(EvaluationArtifactPacketError),
    PacketFileVerificationFailure(EvaluationArtifactPacketError),
    CreatedFileCleanupFailure { kind: io::ErrorKind },
}

pub fn write_encoded_packet_file_create_new(
    path: impl AsRef<Path>,
    encoded: &EncodedEvaluationArtifactPacket,
    limits: EvaluationArtifactPacketFileLimits,
) -> Result<EvaluationArtifactPacketFileWriteReceipt, EvaluationArtifactPacketFileError> {
    let path = path.as_ref();
    validate_encoded_packet_preflight(encoded, limits)?;
    validate_destination_absent(path)?;

    let mut created = false;
    let write_result = (|| -> Result<(), EvaluationArtifactPacketFileError> {
        let mut file = open_create_new_file(path)?;
        created = true;

        file.write_all(&encoded.packet_bytes).map_err(|error| {
            EvaluationArtifactPacketFileError::FileWriteFailure { kind: error.kind() }
        })?;
        file.flush().map_err(
            |error| EvaluationArtifactPacketFileError::FileFlushFailure { kind: error.kind() },
        )?;
        file.sync_all()
            .map_err(|error| EvaluationArtifactPacketFileError::FileSyncFailure {
                kind: error.kind(),
            })?;
        drop(file);

        let reopened = read_regular_file_bytes_bounded(path, limits)?;
        if reopened != encoded.packet_bytes {
            return Err(EvaluationArtifactPacketFileError::PostWriteByteMismatch);
        }

        decode_and_verify_packet(&reopened, Some(&encoded.content_digest)).map_err(|error| {
            EvaluationArtifactPacketFileError::PostWritePacketVerificationFailure(error)
        })?;

        Ok(())
    })();

    if let Err(error) = write_result {
        if created {
            return Err(cleanup_created_file(path, error));
        }
        return Err(error);
    }

    Ok(EvaluationArtifactPacketFileWriteReceipt {
        content_digest: encoded.content_digest.clone(),
        byte_length: encoded.byte_length,
    })
}

pub fn read_and_verify_packet_file(
    path: impl AsRef<Path>,
    expected_digest: Option<&EvaluationArtifactPacketDigest>,
    limits: EvaluationArtifactPacketFileLimits,
) -> Result<VerifiedEvaluationArtifactPacketFile, EvaluationArtifactPacketFileError> {
    let path = path.as_ref();
    let bytes = read_regular_file_bytes_bounded(path, limits)?;
    let verified = decode_and_verify_packet(&bytes, expected_digest)
        .map_err(EvaluationArtifactPacketFileError::PacketFileVerificationFailure)?;
    let content_digest = compute_file_content_digest(&bytes)?;
    let byte_length = u64::try_from(bytes.len())
        .map_err(|_| EvaluationArtifactPacketFileError::FileLengthConversionOverflow)?;

    Ok(VerifiedEvaluationArtifactPacketFile {
        verified_packet: verified,
        content_digest,
        byte_length,
    })
}

fn validate_encoded_packet_preflight(
    encoded: &EncodedEvaluationArtifactPacket,
    limits: EvaluationArtifactPacketFileLimits,
) -> Result<(), EvaluationArtifactPacketFileError> {
    let actual_len = u64::try_from(encoded.packet_bytes.len())
        .map_err(|_| EvaluationArtifactPacketFileError::FileLengthConversionOverflow)?;
    if encoded.byte_length != actual_len {
        return Err(EvaluationArtifactPacketFileError::EncodedPacketLengthMismatch);
    }
    if encoded.byte_length > limits.max_packet_bytes {
        return Err(EvaluationArtifactPacketFileError::EncodedPacketExceedsLimit);
    }

    EvaluationArtifactPacketDigest::new(encoded.content_digest.as_str())
        .map_err(EvaluationArtifactPacketFileError::EncodedPacketVerificationFailure)?;

    decode_and_verify_packet(&encoded.packet_bytes, Some(&encoded.content_digest))
        .map_err(EvaluationArtifactPacketFileError::EncodedPacketVerificationFailure)?;

    Ok(())
}

fn validate_destination_absent(path: &Path) -> Result<(), EvaluationArtifactPacketFileError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(EvaluationArtifactPacketFileError::DestinationAlreadyExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(EvaluationArtifactPacketFileError::FileMetadataFailure { kind: error.kind() })
        }
    }
}

fn open_create_new_file(path: &Path) -> Result<File, EvaluationArtifactPacketFileError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    options.open(path).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            EvaluationArtifactPacketFileError::DestinationAlreadyExists
        } else {
            EvaluationArtifactPacketFileError::FileCreateFailure { kind: error.kind() }
        }
    })
}

fn read_regular_file_bytes_bounded(
    path: &Path,
    limits: EvaluationArtifactPacketFileLimits,
) -> Result<Vec<u8>, EvaluationArtifactPacketFileError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        EvaluationArtifactPacketFileError::FileMetadataFailure { kind: error.kind() }
    })?;

    if metadata.file_type().is_symlink() {
        return Err(EvaluationArtifactPacketFileError::UnsupportedFileType);
    }
    if !metadata.is_file() {
        return Err(EvaluationArtifactPacketFileError::UnsupportedFileType);
    }

    if metadata.len() > limits.max_packet_bytes {
        return Err(EvaluationArtifactPacketFileError::FileExceedsLimit);
    }

    let max_read = limits
        .max_packet_bytes
        .checked_add(1)
        .ok_or(EvaluationArtifactPacketFileError::FileLengthConversionOverflow)?;
    let max_read_usize = usize::try_from(max_read)
        .map_err(|_| EvaluationArtifactPacketFileError::FileLengthConversionOverflow)?;

    let file = File::open(path).map_err(|error| {
        EvaluationArtifactPacketFileError::FileOpenFailure { kind: error.kind() }
    })?;
    let mut buffer = Vec::new();
    file.take(max_read)
        .read_to_end(&mut buffer)
        .map_err(|error| EvaluationArtifactPacketFileError::FileReadFailure {
            kind: error.kind(),
        })?;

    if buffer.len() > max_read_usize.saturating_sub(1) {
        return Err(EvaluationArtifactPacketFileError::FileExceedsLimit);
    }

    Ok(buffer)
}

fn compute_file_content_digest(
    bytes: &[u8],
) -> Result<EvaluationArtifactPacketDigest, EvaluationArtifactPacketFileError> {
    EvaluationArtifactPacketDigest::new(format!("sha256:{}", hex_digest(bytes)))
        .map_err(EvaluationArtifactPacketFileError::PacketFileVerificationFailure)
}

fn hex_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(bytes);
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn cleanup_created_file(
    path: &Path,
    error: EvaluationArtifactPacketFileError,
) -> EvaluationArtifactPacketFileError {
    match fs::remove_file(path) {
        Ok(()) => error,
        Err(cleanup_error) => EvaluationArtifactPacketFileError::CreatedFileCleanupFailure {
            kind: cleanup_error.kind(),
        },
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn zero_limit_rejected() {
        assert_eq!(
            EvaluationArtifactPacketFileLimits::new(0),
            Err(EvaluationArtifactPacketFileError::InvalidFileLimit)
        );
    }
}
