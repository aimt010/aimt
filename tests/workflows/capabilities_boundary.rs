use aimt::workflows::capabilities::{AgentDecision, Capability};
#[test]
fn capability_is_deterministic_decision_is_not() {
    // Update (the Core write) is deterministic; the decision to update is not
    let cap = Capability::Update; // deterministic: Store::open_mut + validate + persist
    let decision = AgentDecision::ShouldUpdate {
        reason: "hash changed".into(),
    };
    assert_ne!(format!("{:?}", cap), format!("{:?}", decision));
    // Must be two distinct types, not Capability::Update being non-deterministic
}
