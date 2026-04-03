# Configuration

This document covers the tunable parts of the public API. Runtime diagnostics such as `ActionScore`, `DecisionTraceBuffer`, and `UtilityAiStats` are intentionally inspectable, but they are outputs rather than inputs.

## Agent-Level Tunables

| Type | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `UtilityAgent::enabled` | `true` | boolean | Disables all evaluation for the agent without despawning its hierarchy |
| `UtilityAgent::selection_strategy` | `HighestScore` | any strategy variant | Chooses how actions are picked once a tier is active |
| `UtilityAgent::selection_seed` | `1` | `u64` | Seed used by randomized strategies and cadence jitter |
| `UtilityAgent::trace_capacity` | `8` | `0..` | Maximum retained action-history entries in `DecisionTraceBuffer` |
| `EvaluationPolicy::mode` | `Interval` | `Interval` or `Manual` | Interval mode self-schedules; manual mode only evaluates on request |
| `EvaluationPolicy::base_interval_seconds` | `0.25` | `>= 0` | Baseline time between evaluations |
| `EvaluationPolicy::jitter_fraction` | `0.0` | `0..=1` | Fractional jitter applied symmetrically around the base interval |
| `EvaluationPolicy::lod_scale` | `1.0` | `>= 0` | Multiplies the base interval for distance or LOD-based throttling |
| `DecisionMomentum::active_action_bonus` | `0.1` | `>= 0` | Bias toward keeping the active action |
| `DecisionMomentum::hysteresis_band` | `0.05` | `>= 0` | Required score gap before switching away from the current action |
| `DecisionMomentum::momentum_decay_per_second` | `0.0` | `>= 0` | Exponential decay rate applied to the active-action bonus |
| `UtilityAiBudget::max_agents_per_update` | `128` | `>= 1` | Frame budget for due-agent evaluation |

## Asset Loading

`UtilityDecisionAssetLoader` registers the `.utility_ai.ron` extension and deserializes full agent/action/consideration graphs.

| Type | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `UtilityDecisionAsset::agent` | n/a | asset-authored | Serializable `UtilityAgent` root data |
| `UtilityDecisionAsset::evaluation_policy` | `None` | optional | Optional serialized `EvaluationPolicy` applied when spawning |
| `UtilityDecisionAsset::momentum` | `None` | optional | Optional serialized `DecisionMomentum` applied when spawning |
| `UtilityDecisionAsset::actions` | empty | any count | Serialized action subtree definitions |

## Action-Level Tunables

| Type | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `UtilityAction::label` | `"action"` | non-empty recommended | Stable human-readable identifier for traces and overlays |
| `UtilityAction::priority` | `PriorityTier::TACTICAL` | any `PriorityTier` | Lower values are considered before higher values |
| `UtilityAction::priority_threshold` | `0.0` | `0..=1` | If any action in a tier reaches its threshold, lower tiers are skipped |
| `UtilityAction::enabled` | `true` | boolean | Hard-disables the action at selection time |
| `UtilityAction::interruptible` | `true` | boolean | Whether a better action may cancel this one |
| `UtilityAction::minimum_commitment_seconds` | `0.0` | `>= 0` | Minimum time before a running action may be interrupted |
| `UtilityAction::weight` | `1.0` | `>= 0` | Scales the action score after target folding |
| `UtilityAction::minimum_score` | `0.0` | `0..=1` | Suppresses the action below the threshold |
| `UtilityAction::composition` | `CompositionPolicy::default()` | see below | Controls how consideration scores compose |
| `UtilityAction::lifecycle` | `Idle` | action lifecycle enum | Runtime execution state owned jointly by the crate and the consumer |
| `UtilityAction::target_requirement` | `Optional` | enum | Marks whether a valid target is mandatory |
| `UtilityAction::target_selection` | `HighestScore` | any strategy variant | Picks the winning target candidate |
| `UtilityAction::target_score_fold` | `Multiply` | enum | Folds the target score into the base action score |
| `UtilityAction::is_fallback` | `false` | boolean | Allows a soft idle/default action to win even when all other actions collapse to zero |

### `CompositionPolicy`

| Field | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `strategy` | `GeometricMean` | any `CompositionStrategy` | Chooses the score-composition algorithm |
| `floor` | `0.0` | typically `0..=1` | Lower bound applied after urgency scaling |
| `ceiling` | `1.0` | typically `0..=1` | Upper bound applied after urgency scaling |
| `urgency_multiplier` | `1.0` | `>= 0` | Final multiplier applied after composition |
| `empty_score` | `0.0` | typically `0..=1` | Score returned when no enabled considerations exist |

### `CompositionStrategy`

| Variant | Use |
| --- | --- |
| `Multiplicative` | Strong veto behavior; one zero kills the score |
| `GeometricMean` | Good general-purpose default with count normalization |
| `Additive` | Weighted averaging when trade-offs should accumulate |
| `Minimum` | Strict weakest-link behavior |
| `CompensatedProduct { compensation_factor }` | Dave Mark style compensation to reduce over-punishment from many considerations |

## Cooldown and Target Tunables

| Type | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `ActionCooldown::duration_seconds` | `0.0` | `>= 0` | Length of the action cooldown |
| `ActionCooldown::restart_on_success` | `true` | boolean | Restart cooldown when the action ends in `Success` |
| `ActionCooldown::restart_on_failure` | `true` | boolean | Restart cooldown when the action ends in `Failure` |
| `ActionCooldown::restart_on_cancel` | `true` | boolean | Restart cooldown when the action ends in `Cancelled` |
| `UtilityTargetCandidate::label` | `"target"` | non-empty recommended | Human-readable candidate identifier for traces and overlays |
| `UtilityTargetCandidate::entity` | `None` | any `Entity` | Optional world entity linked to the candidate |
| `UtilityTargetCandidate::key` | `TargetKey(0)` | any `TargetKey` | Stable external identifier |
| `UtilityTargetCandidate::enabled` | `true` | boolean | Removes the candidate from winning consideration without despawning it |
| `UtilityTargetCandidate::weight` | `1.0` | `>= 0` | Multiplies the candidate-local score |
| `TargetScoreFold::Multiply` | n/a | enum | Multiply base and target scores |
| `TargetScoreFold::Minimum` | n/a | enum | Clamp the action by the weaker of base and target score |
| `TargetScoreFold::Additive { weight }` | n/a | `weight >= 0` | Weighted blend between base and target score |
| `TargetScoreFold::Ignore` | n/a | enum | Use target selection for choice only; leave the base action score unchanged |

## Consideration Tunables

| Type | Default | Valid Range | Effect |
| --- | --- | --- | --- |
| `UtilityConsideration::label` | `"consideration"` | non-empty recommended | Human-readable identifier for traces and overlays |
| `UtilityConsideration::curve` | `Linear` | any `ResponseCurve` | Maps normalized input to normalized utility output |
| `UtilityConsideration::weight` | `1.0` | `>= 0` | Weight used by composition strategies |
| `UtilityConsideration::enabled` | `true` | boolean | Disables the consideration without despawning it |
| `UtilityConsideration::cost` | `Cheap` | enum | Authoring hint for cheap-before-expensive evaluation order |
| `ConsiderationInput::value` | `Some(0.0)` | `0..=1` preferred | Normalized value fed into the response curve |
| `ConsiderationInput::enabled` | `true` | boolean | Disables the input for the current frame |

### `ConsiderationCost`

| Variant | Use |
| --- | --- |
| `Cheap` | Scalar facts already available on the agent |
| `SharedCache` | Inputs read from a cache maintained elsewhere |
| `Expensive` | Inputs that should run late because they are costly to compute |
| `TargetDependent` | Inputs specific to a particular target candidate |

## Selection Strategy Cheat Sheet

| Strategy | Behavior |
| --- | --- |
| `HighestScore` | Deterministic best-score pick |
| `WeightedRandom` | Seeded weighted random over positive scores |
| `TopNRandom { count }` | Seeded random pick within the best `N` scores |
| `ThresholdFirst { threshold }` | First action in declaration order to meet the threshold |
| `TopBandRandom { percent_within_best }` | Seeded weighted pick among scores within a band of the best |

## Response Curve Cheat Sheet

| Curve | Best Use |
| --- | --- |
| `Linear` | Direct proportional scoring |
| `Power { exponent }` | Faster or slower-than-linear ramps |
| `Exponential { power }` | Strong urgency spikes near one end |
| `Logistic { midpoint, steepness }` | Smooth thresholding around a midpoint |
| `InverseLogistic { midpoint, steepness }` | Smooth inverse thresholding |
| `Gaussian { mean, deviation }` | Mid-range sweet spots such as ideal distances |
| `SmoothStep` / `SmootherStep` | Gentle transitions without hard edges |
| `Sine` / `Cosine` | Ease-in or ease-out style shaping |
| `Step { threshold }` | Hard threshold gating |
| `Inverse(inner)` | Invert any base curve |
| `Remap { inner, input_min, input_max }` | Reinterpret a sub-range of the normalized input domain |
| `Clamp { inner, min, max }` | Restrict the input domain before evaluation |
| `Bias { inner, bias }` | Push an existing curve toward one side without replacing it |

## Recommended Usage Notes

- Start with `CompositionStrategy::GeometricMean` unless you have a clear reason to prefer harder veto behavior.
- Use `PriorityTier::CRITICAL` plus a non-zero `priority_threshold` for emergency overrides such as flee, heal, or disengage.
- Prefer `WeightedRandom` or `TopBandRandom` only after you have deterministic scoring working and tested.
- Use `UtilityAction::is_fallback` for idle/default actions, not for emergency actions.
- For large crowds, raise `base_interval_seconds`, add jitter, and keep `UtilityAiBudget` intentionally lower than the total active population.
- If an action is target-dependent, keep target discovery outside this crate and feed the candidate list in explicitly. That preserves deterministic ownership and debug clarity.
