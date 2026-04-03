use std::fmt::{Display, Formatter};

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{
    ActionCooldown, ConsiderationInput, DecisionMomentum, EvaluationPolicy, UtilityAction,
    UtilityAgent, UtilityConsideration,
};

#[derive(Asset, Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct UtilityDecisionAsset {
    #[serde(default)]
    pub agent: UtilityAgent,
    #[serde(default)]
    pub policy: EvaluationPolicy,
    #[serde(default)]
    pub momentum: DecisionMomentum,
    #[serde(default)]
    pub actions: Vec<UtilityActionAsset>,
}

impl UtilityDecisionAsset {
    /// Spawns a utility agent graph from a loaded asset definition and returns the root agent.
    pub fn spawn_in_world(&self, world: &mut World) -> Entity {
        let agent = world
            .spawn((
                Name::new("Utility Decision Agent"),
                self.agent.clone(),
                self.policy.clone(),
                self.momentum.clone(),
            ))
            .id();

        world.entity_mut(agent).with_children(|parent| {
            for action in &self.actions {
                parent
                    .spawn((
                        Name::new(format!("Utility Action: {}", action.action.label)),
                        action.action.clone(),
                        action.cooldown.clone(),
                    ))
                    .with_children(|parent| {
                        for consideration in &action.considerations {
                            parent.spawn((
                                Name::new(format!(
                                    "Utility Consideration: {}",
                                    consideration.consideration.label
                                )),
                                consideration.consideration.clone(),
                                consideration.input.clone(),
                            ));
                        }
                    });
            }
        });

        agent
    }
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct UtilityActionAsset {
    #[serde(default)]
    pub action: UtilityAction,
    #[serde(default)]
    pub cooldown: ActionCooldown,
    #[serde(default)]
    pub considerations: Vec<UtilityConsiderationAsset>,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
pub struct UtilityConsiderationAsset {
    #[serde(default)]
    pub consideration: UtilityConsideration,
    #[serde(default)]
    pub input: ConsiderationInput,
}

#[derive(Default, TypePath)]
pub struct UtilityDecisionAssetLoader;

#[derive(Debug)]
pub enum UtilityDecisionAssetLoaderError {
    Io(std::io::Error),
    Ron(ron::error::SpannedError),
}

impl Display for UtilityDecisionAssetLoaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read utility-ai asset: {error}"),
            Self::Ron(error) => write!(f, "failed to parse utility-ai RON asset: {error}"),
        }
    }
}

impl std::error::Error for UtilityDecisionAssetLoaderError {}

impl From<std::io::Error> for UtilityDecisionAssetLoaderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ron::error::SpannedError> for UtilityDecisionAssetLoaderError {
    fn from(value: ron::error::SpannedError) -> Self {
        Self::Ron(value)
    }
}

impl AssetLoader for UtilityDecisionAssetLoader {
    type Asset = UtilityDecisionAsset;
    type Settings = ();
    type Error = UtilityDecisionAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ron::de::from_bytes::<UtilityDecisionAsset>(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["utility_ai.ron"]
    }
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
