use vox_proof::persistence_evidence::{
    durability_experiments, DirectorySyncCapability, MIN_TRIALS_PER_POINT, TrialOutcome,
};

#[test]
fn durability_experiments_deny_filesystem_and_hardware_power_loss() {
    for spec in durability_experiments() {
        assert!(
            spec.denied.iter().any(|d| *d == "FilesystemDurability"),
            "{} must deny FSD",
            spec.experiment_id
        );
        assert!(
            spec.denied.iter().any(|d| *d == "HardwarePowerLoss"),
            "{} must deny HPL",
            spec.experiment_id
        );
        assert!(
            !spec.sync_boundary.directory_sync_performed,
            "{} must not claim directory sync",
            spec.experiment_id
        );
    }
}

#[test]
fn min_trials_per_point_is_at_least_five() {
    assert!(MIN_TRIALS_PER_POINT >= 5);
}

#[test]
fn directory_sync_capability_not_implemented_by_default() {
    let cap = DirectorySyncCapability::not_implemented();
    assert!(!cap.parent_directory_fsync);
}

#[test]
fn trial_outcome_serializes() {
    let json = serde_json::to_string(&TrialOutcome::Indeterminate).expect("serialize");
    assert!(json.contains("indeterminate"));
}
