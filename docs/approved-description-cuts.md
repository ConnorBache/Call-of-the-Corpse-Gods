# Approved description cuts, 2026-11-16

Base: integrated candidate source v1, SHA256 ff2a76dee92ac37c04ae2faeecea6557621964faec8cf67cf98b14ea48e871ff.

- Clay Army: removed repeated soldier attack explanation; retained `It is an unbreakable wall.`
- Tinker: removed carried-item/automatically-active tutorial only, from description and effect summary. Costs and unique rules retained.
- Arrow item inspection: retained name and quantity only; removed ammunition, capacity, recovery, Drop/Throw tutorial prose.
- Cogworks and potion footer unchanged. No gameplay changes.

Added native regression tests for Clay Army, Tinker and arrows. Native workspace tests attempted but could not execute: `cargo: not found`. These new native tests have NOT run. Source-level Python regression passed: exact catalog changes, all unrelated catalog records unchanged, retained wall/costs, title/quantity-only arrows, potion footer preserved, only intended source files modified.

Prior broad native test results belong to v1, not this copy-only v2. No new native or Windows validation claimed. No GitHub publication.
