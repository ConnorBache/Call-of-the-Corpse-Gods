# Clone description deletion, 2026-11-12

Explicit follow-up approval: when asked whether to remove "This body cannot use that power independently" and keep the final summary "This body shares its powers and energy with you and does not act independently", the designer answered "Yes".

Only the exact suffix " This body cannot use that power independently." was removed from the dynamic template in creature_abilities_ui.rs. The template now remains "A creature acting independently could {}{limit}." Dynamic effect text, placeholders, limits, final detailed summary, compact summary and all code remain unchanged.

This resolves the previous scope hold for review group G-9a346d17c82701ae6d26b884. Original exported labels/comments remain historical records. This is the older R51-based private branch, not missing integrated R52 source. Exact deletion and Rust syntax parsing verified; no Cargo/runtime/visual tests. No main merge.
