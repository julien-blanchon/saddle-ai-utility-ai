use bevy::prelude::*;
use saddle_bevy_e2e::action::Action;

use crate::OverlayText;
use crate::LabDiagnostics;

pub(super) fn overlay_text(world: &mut World) -> Option<String> {
    let mut query = world.query_filtered::<&Text, With<OverlayText>>();
    query.iter(world).next().map(|text| text.0.clone())
}

pub(super) fn overlay_has_labels(world: &mut World, labels: &[&str]) -> bool {
    let overlay = match overlay_text(world) {
        Some(text) => text,
        None => return false,
    };

    labels.iter().all(|label| overlay.contains(label))
}

pub(super) fn wait_for_core_selection() -> Action {
    Action::WaitUntil {
        label: "core utility agents selected".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            !diagnostics.flip_active.is_empty()
                && diagnostics.target_active == "engage"
                && diagnostics.target_selected_target == "relay_beta"
                && diagnostics.priority_active == "panic"
                && diagnostics.action_changed_messages >= 3
        }),
        max_frames: 180,
    }
}

pub(super) fn wait_for_target_selection() -> Action {
    Action::WaitUntil {
        label: "target agent selected relay_beta".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().target_selected_target == "relay_beta"),
        max_frames: 120,
    }
}

pub(super) fn wait_for_priority_selection() -> Action {
    Action::WaitUntil {
        label: "priority agent selected panic".into(),
        condition: Box::new(|world| world.resource::<LabDiagnostics>().priority_active == "panic"),
        max_frames: 120,
    }
}

pub(super) fn wait_for_stress_throughput() -> Action {
    Action::WaitUntil {
        label: "stress throughput ready".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.stress_active_agents >= 40
                && diagnostics.stress_peak_skipped_due_to_budget > 0
                && diagnostics.stress_peak_eval_micros > 0
                && diagnostics.stress_peak_scored_actions >= 40
        }),
        max_frames: 130,
    }
}

pub(super) fn wait_for_handoff_ready() -> Action {
    Action::WaitUntil {
        label: "handoff runtime settled".into(),
        condition: Box::new(|world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.target_selected_target == "relay_beta"
                && diagnostics.priority_active == "panic"
                && diagnostics.stress_peak_skipped_due_to_budget > 0
        }),
        max_frames: 220,
    }
}
