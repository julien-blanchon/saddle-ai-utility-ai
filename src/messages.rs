use bevy::prelude::*;

use crate::components::{ActionChangeReason, ActionLifecycle};

#[derive(Clone, Debug, Message, Reflect)]
pub struct ActionChanged {
    pub agent: Entity,
    pub previous_action: Option<Entity>,
    pub next_action: Option<Entity>,
    pub previous_label: Option<String>,
    pub next_label: Option<String>,
    pub reason: ActionChangeReason,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct ActionCompleted {
    pub agent: Entity,
    pub action: Entity,
    pub label: String,
    pub lifecycle: ActionLifecycle,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct ActionEvaluationRequested {
    pub agent: Entity,
    pub reason: String,
}

impl ActionEvaluationRequested {
    pub fn new(agent: Entity, reason: impl Into<String>) -> Self {
        Self {
            agent,
            reason: reason.into(),
        }
    }
}
