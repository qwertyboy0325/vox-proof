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
    CorruptDerivedArtifact,
    CorruptCanonicalPayload,
    SetUnknownNewerFormat,
    InterruptCompaction,
    InterruptCleanup,
}

#[derive(Debug, Clone)]
pub struct PendingFault {
    pub scenario_id: String,
    pub point: FaultPoint,
    pub layer: FaultLayer,
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
}
