# Ordered divine rescue helpers

`divine_rescues.rs` mirrors the prevention segment of frozen combat.ts death and the Overflowing counter segment of damage.144 source vectors verify acquisition-order precedence, Rage affordability and0→1 doubling, failed-toggle disabling/fallback, Pelian spent state and bypass.64 source packet vectors verify fractional legacy damage banks, thresholds and55-blood cap.

Call `try_rescue` after establishing nonpositive authoritative owner HP and before clearing Beat to Death or recording terminal death. It returns without preventing crushing when bypassed. It does not create a terminal corpse/death record; the existing lifecycle owns that. Rage resource history is paid through the normal shared-pool payment function. Rescues sync linked bodies without separate resource pools.

Call `accrue_damage` after shared HP loss and before prevention. The full resolved integer packet counts, including overkill; it is not capped to actual remaining HP. Do not accrue prevented/immune/zero packets.

Complexity: A current actors, K owned ability entries, D resolved damage. Rescue costs O(K×A) with the existing stable-ID linear lookup and O(K) cloned ordering storage, plus O(A) synchronization. Overflowing costs O(A+K+D/50) time and O(1) extra storage; repeated subtraction deliberately preserves source floating-point behavior of legacy fractional banks. Actual validated game damage is bounded by gameplay state, but this helper is not claimed constant-time for arbitrary numeric input.

The ability owner supplies activation/acquisition/level reset; combat supplies exact damage/death call sites. Helper vectors alone do not certify complete command/event/history interactions. All terminal behavior and linked bodies require their integration suites.
