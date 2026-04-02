use bevy::prelude::*;
use bevy_e2e::action::Action;
use saddle_ai_utility_ai::UtilityAiSystems;

use crate::scenarios;

pub struct UtilityAiLabE2EPlugin;

impl Plugin for UtilityAiLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_e2e::E2EPlugin);
        app.configure_sets(
            Update,
            bevy_e2e::E2ESet.before(UtilityAiSystems::GatherInputs),
        );

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenarios::scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                bevy_e2e::init_scenario(app, scenario);
            } else {
                error!(
                    "[utility_ai_lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    scenarios::list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}
