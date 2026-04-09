mod support;

use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

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
        .then(support::wait_for_core_selection())
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert!(diagnostics.action_changed_messages >= 3);
            assert_eq!(diagnostics.target_selected_target, "relay_beta");
            assert_eq!(diagnostics.priority_active, "panic");

            assert!(support::overlay_has_labels(
                world,
                &["utility_ai lab", "flip agent", "target agent", "priority agent"],
            ));
        })))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(name))
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
        .then(assertions::log_summary("utility_ai_flip_flop"))
        .build()
}

fn utility_ai_target_pick() -> Scenario {
    Scenario::builder("utility_ai_target_pick")
        .description("Verify the target-scored action selects the expected relay target and exposes the winner in diagnostics.")
        .then(support::wait_for_target_selection())
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
        .then(assertions::log_summary("utility_ai_target_pick"))
        .build()
}

fn utility_ai_priority_tiers() -> Scenario {
    Scenario::builder("utility_ai_priority_tiers")
        .description("Verify a critical-tier emergency action suppresses a strong tactical option once its threshold is met.")
        .then(support::wait_for_priority_selection())
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
        .then(assertions::log_summary("utility_ai_priority_tiers"))
        .build()
}

fn utility_ai_stress() -> Scenario {
    Scenario::builder("utility_ai_stress")
        .description("Verify crowd evaluation stays budgeted across frames and the stress swarm produces real throughput diagnostics.")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("stress_warmup".into()))
        .then(support::wait_for_stress_throughput())
        .then(Action::Custom(Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            assert!(diagnostics.stress_active_agents >= 40);
            assert!(diagnostics.stress_peak_skipped_due_to_budget > 0);
            assert!(diagnostics.stress_peak_eval_micros > 0);
            assert!(diagnostics.stress_peak_scored_actions >= 40);
        })))
        .then(Action::Screenshot("stress_peak".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("utility_ai_stress"))
        .build()
}

fn utility_ai_handoff() -> Scenario {
    Scenario::builder("utility_ai_handoff")
        .description("Leave the lab running after the core scenarios settle so BRP can inspect traces, scores, and the active target line.")
        .then(support::wait_for_handoff_ready())
        .then(Action::Screenshot("handoff_ready".into()))
        .handoff()
        .build()
}
