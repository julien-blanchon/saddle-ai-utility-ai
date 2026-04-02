# Utility AI Lab

Crate-local standalone lab app for validating the shared `saddle-ai-utility-ai` crate in a live Bevy scene.

## Purpose

- exercise momentum, hysteresis, target scoring, priority tiers, and budgeted crowd evaluation in one runtime
- expose stable named entities, a diagnostics resource, and BRP-readable trace data for fast inspection
- keep richer BRP and E2E verification inside the shared crate instead of leaking it into project sandboxes or `crates/e2e`

## Status

Working

## Run

```bash
cargo run -p saddle-ai-utility-ai-lab
```

## E2E

```bash
cargo run -p saddle-ai-utility-ai-lab --features e2e -- smoke_launch
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_smoke
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_flip_flop
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_target_pick
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_priority_tiers
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_stress
cargo run -p saddle-ai-utility-ai-lab --features e2e -- utility_ai_handoff
```

## BRP

```bash
uv run --active --project .codex/skills/bevy-brp/script brp app launch saddle-ai-utility-ai-lab
uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_utility_ai::UtilityAgent
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_utility_ai::ActiveAction
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_utility_ai::ActionScore
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_utility_ai::ActionTarget
uv run --active --project .codex/skills/bevy-brp/script brp resource get saddle_ai_utility_ai_lab::LabDiagnostics
uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle_ai_utility_ai_lab.png
uv run --active --project .codex/skills/bevy-brp/script brp extras shutdown
```

## Notes

- The lab uses simple generated geometry on purpose. The verification target is decision quality and inspectability, not authored art.
- `utility_ai_handoff` leaves the app running after the showcase settles so BRP can inspect traces, scores, and the winning target line without rerunning the scenario.
