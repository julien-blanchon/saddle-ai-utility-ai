use super::*;

#[test]
fn decision_trace_defaults_empty() {
    let trace = DecisionTrace::default();
    assert!(trace.winning_action.is_none());
    assert!(trace.actions.is_empty());
    assert!(trace.recent_history.is_empty());
}

#[test]
fn action_trace_carries_suppression_reason() {
    let trace = ActionTrace {
        label: "fallback".into(),
        suppression: Some(ActionSuppressionReason::Cooldown),
        ..default()
    };
    assert_eq!(trace.suppression, Some(ActionSuppressionReason::Cooldown));
}

#[test]
fn target_candidate_trace_keeps_consideration_breakdown() {
    let trace = TargetCandidateTrace {
        label: "target".into(),
        considerations: vec![ConsiderationTrace {
            label: "distance".into(),
            output: 0.75,
            ..default()
        }],
        ..default()
    };

    assert_eq!(trace.considerations.len(), 1);
    assert_eq!(trace.considerations[0].label, "distance");
}
