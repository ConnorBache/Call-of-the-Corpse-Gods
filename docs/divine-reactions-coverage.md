# Divine reaction decision helpers

`divine_reactions.rs` provides Never Surrender, Quick Hand, I Hit Back, Are You Sure About That?, Waking Nightmare and Bleed Them decisions. Engine owns recursive attacks/damage and exact source hook order.

Verified locally:96 source Nightmare predicates,72 source Surrender strike projections, plus reaction/payment/deflection/zero-damage/fractional-state boundaries. Abilities integration additionally reports12 scenarios/84 full-state and ordered-event commands; these are covered paths, not every possible combination.

Important source semantics:

- Missing surrenderTick differs from tick0. Protection may use negative Defense; do not silently clamp it. Payment occurs only when the ordinary unprotected landed strike would damage.
- Quick Hand can trigger on a landed zero-damage hit while the body survives. Haste freshness depends on resolvingTick.
- I Hit Back uses pre-strike defender Offense and current attacker mitigation, may occur after a terminal hit, and consumes its once-per-turn marker even for a zero packet. Recursive damage is secondary.
- Deflection candidates preserve player/current-actor order and use raw adjacency/hostility. The original attacker can be selected. An empty candidate set consumes neither reaction nor RNG.
- Nightmare absorption needs a hostile source and a funded nightmare counter, after packet validity/immunity checks. Shared bodies use one counter.
- Bleed Them records even a zero-duration bleeding field/source ID on a positive landed packet when Offense is nonpositive, matching source state.

Costs: A = current actors, K = abilities/actives. Owner resolution is O(A); predicates are O(A+K). Deflection selection is O(A+K) time and O(A) candidate storage, then one RNG draw. Shared-body synchronization is O(A+endpoint metadata). These helpers perform no all-floor traversal, serialization or recursive attacks. The caller's combat/effective-stat costs are additional.

R52 [Never Surrender clarification and preview audit](revision52-never-surrender-audit.md): DR-bypassing strikes no longer spend or consume its proc; ordinary melee/ranged and full-block payment are covered. Direct/status packets already bypass DR and leave the proc available. Reserve-aware attack forecasts and payment before Hanged share eligibility. Legacy gameplay and ordered rescues are unchanged.
