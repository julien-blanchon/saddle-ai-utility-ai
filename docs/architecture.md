# Architecture

`saddle-ai-utility-ai` is split into a pure scoring core plus a thin Bevy adapter so the math stays easy to unit test and the runtime stays easy to inspect.

## Module Layout

- `curves.rs`: normalized response-curve families and wrappers
- `scoring.rs`: consideration weighting and composition strategies
- `selection.rs`: deterministic and opt-in randomized pickers
- `momentum.rs`: hysteresis and active-action bonus helpers
- `tracing.rs`: structured explanation data used by BRP, overlays, and E2E assertions
- `systems.rs`: hierarchy traversal, cadence, scoring, selection, transitions, and trace refresh

## Why This ECS Shape

The crate uses Bevy's built-in parent/child hierarchy because it keeps the runtime debuggable through standard `Children` traversal and BRP queries.

```text
Agent
├── UtilityAgent
├── EvaluationPolicy
├── ActiveAction
├── DecisionMomentum
├── DecisionTraceBuffer
└── Action
    ├── UtilityAction
    ├── ActionScore
    ├── ActionCooldown
    ├── ActionTarget
    ├── Consideration
    │   ├── UtilityConsideration
    │   └── ConsiderationInput
    └── TargetCandidate
        ├── UtilityTargetCandidate
        └── Consideration*
```

Why this shape:

- agent state is centralized on the agent entity, so cadence and current-action inspection are one query away
- action-local state lives on action children, so game code can add, remove, or retune actions without touching the agent root
- target candidates stay under their owning action, which keeps target scoring local instead of requiring a global registry or blackboard
- consideration inputs remain plain data components, so game code can fill them from any sensor or world query without callbacks stored in ECS

## Evaluation Pipeline

`UtilityAiSystems` always run in this order:

1. `GatherInputs`
2. `Score`
3. `Select`
4. `Transition`
5. `DebugRender`

### `GatherInputs`

- bootstrap newly added agents and actions with missing runtime components
- read `ActionEvaluationRequested` messages and mark the agent as due
- tick cooldown timers
- sort due agents by deadline and apply the frame budget

### `Score`

- sort considerations from cheap to expensive using `ConsiderationCost`
- evaluate action-level considerations into a base score
- evaluate optional target-candidate subtrees
- fold the winning target score into the action score
- store per-action traces directly on `ActionScore`

### `Select`

- group eligible actions by `PriorityTier`
- stop at the first tier whose threshold is satisfied
- apply the agent's `SelectionStrategy` using the seeded deterministic picker helpers
- store the provisional result in an internal `PendingSelection`

### `Transition`

- finish terminal actions and emit `ActionCompleted`
- compare the pending winner against the current action
- respect interruptibility, minimum commitment, hysteresis, and cooldown restarts
- emit `ActionChanged` when the active action actually changes
- compute the next evaluation deadline and snapshot a `DecisionTrace`

### `DebugRender`

- trim trace history to the agent's configured retention capacity
- keep the latest trace ready for BRP, UI overlays, and E2E assertions

## Target Scoring Flow

Target scoring is intentionally separate from target discovery.

1. Game code spawns or updates `UtilityTargetCandidate` children under an action.
2. Candidate-local considerations produce one candidate score each.
3. The action's `target_selection` strategy picks the winning candidate.
4. `TargetScoreFold` merges the target score back into the action score.
5. Required-target actions are suppressed when no valid candidate survives.

This separation keeps the crate reusable across genres: one consumer can score enemies, another can score workstations, cover points, or waypoints without changing the runtime.

## Momentum and Cooldown Flow

- `DecisionMomentum::active_action_bonus` gives the current action a configurable stickiness bonus
- `DecisionMomentum::momentum_decay_per_second` can taper that bonus over time
- `DecisionMomentum::hysteresis_band` blocks small score improvements from causing a switch
- `UtilityAction::minimum_commitment_seconds` prevents immediate interruptions after a switch
- `ActionCooldown` handles post-success, post-failure, and post-cancel lockout windows

The crate deliberately keeps these rules in the transition step rather than the score step so the raw utility traces remain visible even when the final action is held for stability reasons.

## Trace and Debug Model

Every evaluated action writes inspection-friendly data into `ActionScore`:

- base, target, final, and momentum-adjusted scores
- suppression reason
- per-consideration inputs and outputs
- per-target candidate scores

Every evaluated agent writes a `DecisionTrace` snapshot into `DecisionTraceBuffer`:

- winning action and winning target
- evaluation and next-evaluation timestamps
- whether the evaluation was explicitly requested
- whether the frame budget caused spillover
- a bounded recent action-history ring buffer

The runtime-facing public components are reflective, so BRP can inspect both the live decision state and the latest score outputs directly.

## Performance Model

The runtime is designed around predictable single-threaded behavior first.

- agents evaluate on cadence, not every frame
- `UtilityAiBudget` caps how many due agents are processed in one update
- due agents are sorted so overdue work is serviced first
- consideration ordering is cheap-first, and multiplicative actions short-circuit once a zero score is encountered
- `UtilityAiStats` exposes evaluation counts, target counts, budget spillover, and timing for overlays or automated checks

Async or parallel evaluation is intentionally out of scope for v0.1. Consumers should first confirm that cadence, budgets, and consideration ordering are insufficient before pushing the design toward background work.
