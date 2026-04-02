mod support;

use saddle_bevy_e2e::{action::Action, scenario::Scenario};

use crate::LabDiagnostics;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "utility_ai_smoke",
        "utility_ai_flip_flop",
        "utility_ai_target_pick",
        "utility_ai_priority_tiers",
        "utility_ai_stress",
        "utility_ai_handoff",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(build_smoke("smoke_launch")),
        "utility_ai_smoke" => Some(build_smoke("utility_ai_smoke")),
        "utility_ai_flip_flop" => Some(utility_ai_flip_flop()),
        "utility_ai_target_pick" => Some(utility_ai_target_pick()),
        "utility_ai_priority_tiers" => Some(utility_ai_priority_tiers()),
        "utility_ai_stress" => Some(utility_ai_stress()),
        "utility_ai_handoff" => Some(utility_ai_handoff()),
        _ => None,
    }
}

fn build_smoke(name: &'static str) -> Scenario {
    Scenario::builder(name)
        .description("Boot the crate-local utility AI lab, wait for all showcase agents to pick actions, and capture the diagnostic overlay.")
        .then(Action::WaitUntil {
            label: "agents selected".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                !diagnostics.flip_active.is_empty()
                    && diagnostics.target_active == "engage"
                    && diagnostics.target_selected_target == "relay_beta"
                    && diagnostics.priority_active == "panic"
                    && diagnostics.action_changed_messages >= 3
            }),
            max_frames: 180,
        })
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert!(diagnostics.action_changed_messages >= 3);
            assert_eq!(diagnostics.target_selected_target, "relay_beta");
            assert_eq!(diagnostics.priority_active, "panic");

            let overlay = support::overlay_text(world).expect("overlay text should exist");
            assert!(overlay.contains("utility_ai lab"));
            assert!(overlay.contains("flip agent"));
            assert!(overlay.contains("target agent"));
            assert!(overlay.contains("priority agent"));
        })))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn utility_ai_flip_flop() -> Scenario {
    Scenario::builder("utility_ai_flip_flop")
        .description("Verify hysteresis and commitment keep the oscillating agent from thrashing between near-equal actions.")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("flip_flop_early".into()))
        .then(Action::WaitFrames(150))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert!(diagnostics.flip_switch_count <= 5);
            assert!(
                diagnostics.flip_active == "advance" || diagnostics.flip_active == "hold",
                "unexpected flip agent action: {}",
                diagnostics.flip_active
            );
        })))
        .then(Action::Screenshot("flip_flop_late".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn utility_ai_target_pick() -> Scenario {
    Scenario::builder("utility_ai_target_pick")
        .description("Verify the target-scored action selects the expected relay target and exposes the winner in diagnostics.")
        .then(Action::WaitUntil {
            label: "target selected".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().target_selected_target == "relay_beta"),
            max_frames: 120,
        })
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert_eq!(diagnostics.target_active, "engage");
            assert_eq!(diagnostics.target_selected_target, "relay_beta");
            assert!(diagnostics.target_selected_score > 0.85);
        })))
        .then(Action::Screenshot("target_pick_settled".into()))
        .then(Action::WaitFrames(45))
        .then(Action::Screenshot("target_pick_hold".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn utility_ai_priority_tiers() -> Scenario {
    Scenario::builder("utility_ai_priority_tiers")
        .description("Verify a critical-tier emergency action suppresses a strong tactical option once its threshold is met.")
        .then(Action::WaitUntil {
            label: "priority settled".into(),
            condition: Box::new(|world| world.resource::<LabDiagnostics>().priority_active == "panic"),
            max_frames: 120,
        })
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert_eq!(diagnostics.priority_active, "panic");
            assert!(diagnostics.priority_emergency_score >= 0.75);
            assert!(diagnostics.priority_tactical_score >= 0.9);
            assert!(diagnostics.priority_tactical_suppressed);
        })))
        .then(Action::Screenshot("priority_tiers_settled".into()))
        .then(Action::WaitFrames(45))
        .then(Action::Screenshot("priority_tiers_hold".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn utility_ai_stress() -> Scenario {
    Scenario::builder("utility_ai_stress")
        .description("Verify crowd evaluation stays budgeted across frames and the stress swarm produces real throughput diagnostics.")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("stress_warmup".into()))
        .then(Action::WaitFrames(130))
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert!(diagnostics.stress_active_agents >= 40);
            assert!(diagnostics.stress_peak_skipped_due_to_budget > 0);
            assert!(diagnostics.stress_peak_eval_micros > 0);
            assert!(diagnostics.stress_peak_scored_actions >= 40);
        })))
        .then(Action::Screenshot("stress_peak".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn utility_ai_handoff() -> Scenario {
    Scenario::builder("utility_ai_handoff")
        .description("Leave the lab running after the core scenarios settle so BRP can inspect traces, scores, and the active target line.")
        .then(Action::WaitUntil {
            label: "stable runtime".into(),
            condition: Box::new(|world| {
                let diagnostics = world.resource::<LabDiagnostics>();
                diagnostics.target_selected_target == "relay_beta"
                    && diagnostics.priority_active == "panic"
                    && diagnostics.stress_peak_skipped_due_to_budget > 0
            }),
            max_frames: 220,
        })
        .then(Action::Screenshot("handoff_ready".into()))
        .handoff()
        .build()
}
