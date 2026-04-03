use bevy::prelude::*;

use super::*;
use crate::{ResponseCurve, UtilityAction, UtilityConsideration};

#[test]
fn utility_decision_asset_spawns_agent_graph() {
    let asset = UtilityDecisionAsset {
        actions: vec![UtilityActionAsset {
            action: UtilityAction::new("advance"),
            considerations: vec![UtilityConsiderationAsset {
                consideration: UtilityConsideration::new("need", ResponseCurve::Linear),
                input: crate::ConsiderationInput {
                    value: Some(0.8),
                    enabled: true,
                },
            }],
            ..default()
        }],
        ..default()
    };

    let mut world = World::new();
    let agent = asset.spawn_in_world(&mut world);

    let action_entities = world.get::<Children>(agent).unwrap().to_vec();
    assert_eq!(action_entities.len(), 1);

    let consideration_entities = world.get::<Children>(action_entities[0]).unwrap().to_vec();
    assert_eq!(consideration_entities.len(), 1);
    assert!(world.get::<UtilityAction>(action_entities[0]).is_some());
    assert!(world
        .get::<UtilityConsideration>(consideration_entities[0])
        .is_some());
}
