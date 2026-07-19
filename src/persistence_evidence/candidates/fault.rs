use std::cell::RefCell;
use std::collections::BTreeSet;

use super::super::scenario::ScenarioIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultLayer {
    Logical,
    Filesystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FaultExecutionMode {
    #[default]
    ReturnError,
    ProcessAbort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultPoint {
    FailBeforeDurabilityCommit,
    BeforeSqliteCommit,
    AfterSqliteCommitBeforeAck,
    DuringBackupCopy,
    AfterBackupBeforeDestinationIdentityChange,
    AfterDestinationIdentityChangeBeforePublish,
    AfterPublishBeforeReturn,
    DuringCheckpoint,
    CorruptDerivedArtifact,
    CorruptCanonicalPayload,
    SetUnknownNewerFormat,
    InterruptCompaction,
    InterruptCleanup,
}

impl FaultPoint {
    pub fn fault_id(self) -> &'static str {
        match self {
            Self::FailBeforeDurabilityCommit => "fail_before_durability_commit",
            Self::BeforeSqliteCommit => "before_sqlite_commit",
            Self::AfterSqliteCommitBeforeAck => "after_sqlite_commit_before_ack",
            Self::DuringBackupCopy => "during_backup_copy",
            Self::AfterBackupBeforeDestinationIdentityChange => {
                "after_backup_before_destination_identity_change"
            }
            Self::AfterDestinationIdentityChangeBeforePublish => {
                "after_destination_identity_change_before_publish"
            }
            Self::AfterPublishBeforeReturn => "after_publish_before_return",
            Self::DuringCheckpoint => "during_checkpoint",
            Self::CorruptDerivedArtifact => "corrupt_derived_artifact",
            Self::CorruptCanonicalPayload => "corrupt_canonical_payload",
            Self::SetUnknownNewerFormat => "set_unknown_newer_format",
            Self::InterruptCompaction => "interrupt_compaction",
            Self::InterruptCleanup => "interrupt_cleanup",
        }
    }

    pub fn authority_changed_before_fault(self) -> bool {
        match self {
            Self::FailBeforeDurabilityCommit
            | Self::BeforeSqliteCommit
            | Self::DuringBackupCopy
            | Self::AfterBackupBeforeDestinationIdentityChange
            | Self::CorruptDerivedArtifact
            | Self::CorruptCanonicalPayload
            | Self::SetUnknownNewerFormat
            | Self::InterruptCompaction
            | Self::InterruptCleanup => false,
            Self::AfterSqliteCommitBeforeAck
            | Self::AfterDestinationIdentityChangeBeforePublish
            | Self::AfterPublishBeforeReturn
            | Self::DuringCheckpoint => true,
        }
    }

    pub fn from_env_id(id: &str) -> Option<Self> {
        match id {
            "fail_before_durability_commit" => Some(Self::FailBeforeDurabilityCommit),
            "before_sqlite_commit" => Some(Self::BeforeSqliteCommit),
            "after_sqlite_commit_before_ack" => Some(Self::AfterSqliteCommitBeforeAck),
            "during_backup_copy" => Some(Self::DuringBackupCopy),
            "after_backup_before_destination_identity_change" => {
                Some(Self::AfterBackupBeforeDestinationIdentityChange)
            }
            "after_destination_identity_change_before_publish" => {
                Some(Self::AfterDestinationIdentityChangeBeforePublish)
            }
            "after_publish_before_return" => Some(Self::AfterPublishBeforeReturn),
            "during_checkpoint" => Some(Self::DuringCheckpoint),
            "interrupt_compaction" => Some(Self::InterruptCompaction),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingFault {
    pub scenario_id: String,
    pub point: FaultPoint,
    pub layer: FaultLayer,
    pub authority_changed_before_fault: bool,
    pub execution_mode: FaultExecutionMode,
}

#[derive(Default)]
pub struct FaultRegistry {
    pending: RefCell<BTreeSet<(String, FaultPoint)>>,
    armed: RefCell<Option<PendingFault>>,
    execution_mode: RefCell<FaultExecutionMode>,
}

impl FaultRegistry {
    pub fn set_execution_mode(&self, mode: FaultExecutionMode) {
        *self.execution_mode.borrow_mut() = mode;
    }

    pub fn execution_mode(&self) -> FaultExecutionMode {
        *self.execution_mode.borrow()
    }

    pub fn arm_for_scenario(
        &self,
        scenario: &ScenarioIdentity,
        point: FaultPoint,
        layer: FaultLayer,
    ) {
        self.pending
            .borrow_mut()
            .insert((scenario.scenario_id.clone(), point));
        self.armed.borrow_mut().replace(PendingFault {
            scenario_id: scenario.scenario_id.clone(),
            point,
            layer,
            authority_changed_before_fault: point.authority_changed_before_fault(),
            execution_mode: self.execution_mode(),
        });
    }

    pub fn arm_for_test(&self, point: FaultPoint) {
        self.armed.borrow_mut().replace(PendingFault {
            scenario_id: "mechanism-test".to_string(),
            point,
            layer: FaultLayer::Logical,
            authority_changed_before_fault: point.authority_changed_before_fault(),
            execution_mode: self.execution_mode(),
        });
    }

    pub fn arm_point(&self, point: FaultPoint, execution_mode: FaultExecutionMode) {
        self.armed.borrow_mut().replace(PendingFault {
            scenario_id: "process-worker".to_string(),
            point,
            layer: FaultLayer::Logical,
            authority_changed_before_fault: point.authority_changed_before_fault(),
            execution_mode,
        });
    }

    pub fn take_if_armed(&self, point: FaultPoint) -> Option<PendingFault> {
        let armed = self.armed.borrow().clone();
        if armed.as_ref().is_some_and(|fault| fault.point == point) {
            self.armed.borrow_mut().take();
            return armed;
        }
        None
    }

    pub fn clear(&self) {
        self.pending.borrow_mut().clear();
        self.armed.borrow_mut().take();
        *self.execution_mode.borrow_mut() = FaultExecutionMode::default();
    }
}

pub fn handle_fault(fault: PendingFault) -> ! {
    match fault.execution_mode {
        FaultExecutionMode::ProcessAbort => std::process::abort(),
        FaultExecutionMode::ReturnError => {
            panic!(
                "handle_fault called with ReturnError mode for {:?}",
                fault.point
            );
        }
    }
}
