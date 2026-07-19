use std::cell::RefCell;
use std::collections::BTreeSet;

use super::super::scenario::ScenarioIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultLayer {
    Logical,
    Filesystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultPoint {
    FailBeforeDurabilityCommit,
    BeforeSqliteCommit,
    AfterSqliteCommitBeforeAck,
    DuringBackupCopy,
    DuringCheckpoint,
    CorruptDerivedArtifact,
    CorruptCanonicalPayload,
    SetUnknownNewerFormat,
    InterruptCompaction,
    InterruptCleanup,
}

impl FaultPoint {
    pub fn authority_changed_before_fault(self) -> bool {
        match self {
            Self::FailBeforeDurabilityCommit
            | Self::BeforeSqliteCommit
            | Self::DuringBackupCopy
            | Self::CorruptDerivedArtifact
            | Self::CorruptCanonicalPayload
            | Self::SetUnknownNewerFormat
            | Self::InterruptCompaction
            | Self::InterruptCleanup => false,
            Self::AfterSqliteCommitBeforeAck | Self::DuringCheckpoint => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingFault {
    pub scenario_id: String,
    pub point: FaultPoint,
    pub layer: FaultLayer,
    pub authority_changed_before_fault: bool,
}

#[derive(Default)]
pub struct FaultRegistry {
    pending: RefCell<BTreeSet<(String, FaultPoint)>>,
    armed: RefCell<Option<PendingFault>>,
}

impl FaultRegistry {
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
    }

    pub fn arm_for_test(&self, point: FaultPoint) {
        self.armed.borrow_mut().replace(PendingFault {
            scenario_id: "mechanism-test".to_string(),
            point,
            layer: FaultLayer::Logical,
            authority_changed_before_fault: point.authority_changed_before_fault(),
        });
    }
}
