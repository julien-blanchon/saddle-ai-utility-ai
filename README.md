# Saddle AI Utility AI

Reusable utility-based decision making for Bevy ECS agents.

The crate separates pure utility math from ECS orchestration. Game code owns perception, target discovery, locomotion, combat, animation, and any project-specific world queries. `utility_ai` owns response curves, consideration scoring, target folding, action selection, cooldowns, momentum, evaluation cadence, and runtime debug traces.

For apps where utility agents should stay active for the full app lifetime, prefer `UtilityAiPlugin::always_on(Update)`. Use `UtilityAiPlugin::new(...)` when activation should follow explicit schedules such as `OnEnter` / `OnExit`.

## Quick Start

```toml
[dependencies]
saddle-ai-utility-ai = { git = "https://github.com/julien-blanchon/saddle-ai-utility-ai" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_ai_utility_ai::{
    ActionEvaluationRequested, ActionScore, ConsiderationInput, EvaluationPolicy, PriorityTier,
    ResponseCurve, UtilityAction, UtilityAgent, UtilityAiPlugin, UtilityConsideration,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UtilityAiPlugin::always_on(Update))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Agent"),
            UtilityAgent::default(),
            EvaluationPolicy::interval(0.2),
        ))
        .with_children(|agent| {
            agent
                .spawn((Name::new("Advance"), UtilityAction::new("advance").with_priority(PriorityTier::TACTICAL, 0.4)))
                .with_children(|action| {
                    action.spawn((
                        Name::new("Opportunity"),
                        UtilityConsideration::new("opportunity", ResponseCurve::SmoothStep),
                        ConsiderationInput { value: Some(0.8), enabled: true },
                    ));
                });
        });
}
```

## Public API

- Plugin: `UtilityAiPlugin`
- System sets:
  `UtilityAiSystems::{GatherInputs, Score, Select, Transition, DebugRender}`
- Core components:
  `UtilityAgent`, `EvaluationPolicy`, `DecisionMomentum`, `ActiveAction`
- Action components:
  `UtilityAction`, `ActionScore`, `ActionCooldown`, `ActionTarget`, `UtilityTargetCandidate`
- Consideration components:
  `UtilityConsideration`, `ConsiderationInput`
- Resources:
  `UtilityAiBudget`, `UtilityAiStats`
- Pure decision types:
  `ResponseCurve`, `CompositionPolicy`, `CompositionStrategy`, `SelectionStrategy`, `PriorityTier`, `DecisionTrace`
- Messages:
  `ActionChanged`, `ActionCompleted`, `ActionEvaluationRequested`

## Core Model

- Agents are parent entities that own cadence, momentum, debug history, and selection policy.
- Actions are child entities that own lifecycle, priority, scoring policy, cooldowns, and target policy.
- Considerations are child entities under an action or a target-candidate entity. Game systems fill `ConsiderationInput`; the crate only evaluates the normalized values.
- Target candidates are optional action children. They can carry their own consideration children, letting the crate score zero-target, single-target, and multi-target actions with the same runtime.
- `ActionScore` stores the latest consideration traces, target traces, suppression reason, and final score so BRP and E2E checks can inspect decisions directly.

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal single-agent utility loop | `cargo run -p saddle-ai-utility-ai-example-basic` |
| `response_curves` | Visualize built-in curve families | `cargo run -p saddle-ai-utility-ai-example-response-curves` |
| `selection_strategies` | Compare selection strategies side-by-side | `cargo run -p saddle-ai-utility-ai-example-selection-strategies` |
| `target_scoring` | Pick the best target for a target-dependent action | `cargo run -p saddle-ai-utility-ai-example-target-scoring` |
| `realtime_cadence` | Show staggered cadence, jitter, and reevaluation requests | `cargo run -p saddle-ai-utility-ai-example-realtime-cadence` |
| `stress_test` | Spawn many agents and log utility throughput | `cargo run -p saddle-ai-utility-ai-example-stress-test` |
| `saddle-ai-utility-ai-lab` | Crate-local BRP/E2E showcase app | `cargo run -p saddle-ai-utility-ai-lab` |

## Crate-Local Lab

`shared/ai/saddle-ai-utility-ai/examples/lab` is the richer verification surface for this crate. It keeps the BRP wiring, debug overlay, throughput diagnostics, and targeted E2E scenarios inside the shared crate instead of relying on a project-level sandbox.

```bash
cargo run -p saddle-ai-utility-ai-lab
```

E2E commands:

```bash
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_smoke
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_flip_flop
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_target_pick
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_priority_tiers
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_stress
```

Lab guide:

- [Lab README](examples/lab/README.md)

## Limitations

- The crate does not gather world data. Consuming game systems must write normalized `ConsiderationInput` values and manage target candidate lifetimes.
- The v0.1 target model is hierarchy-based and optimized for ECS inspection, not for large shared blackboard graphs.
- Randomized selection is opt-in. Deterministic strategies are the default.
- The crate does not ship a genre-specific action executor. Consumers own the actual behavior behind `Requested`, `Executing`, `Success`, `Failure`, and `Cancelled`.

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
