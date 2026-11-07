# Divine combat helper slice

Owned helpers in `src/divine_combat.rs` implement Fight Long, Beat to Death, Hardened Blood and Blood Weapon from frozen v48 combat.ts. They do not execute recursive attacks themselves.

## Verified helper evidence

-192 source vectors cover player, enemy, Mini-Me and Half Here stat projections, derived maximum health, missing-health DR, accumulated negative armor/dodge, HP payment rounding, readiness ownership and Reverse Entropy healing suppression.
- Narrow mutation tests cover living versus dead-target debuff, ranged versus melee healing, fractional healing, cap, inability to heal a dead owner, and one-time Blood and Fury achievement logging.
- Actual activation and attack-phase tests are owned by the abilities/combat integration suites. Passing these helpers alone does not establish full command ordering, rescues, attack blockers, recursive reactions, offhand or clone-removal interactions.

## Ordering invariants

Beat penalties apply only after a landed strike, including zero damage, while the target remains alive. Hardened Blood uses the same derived maximum as the health pool without recursively calculating its own DR. Blood Weapon is paid from maximum HP, rounds damage upward, and consumes its prime on a resolved hit or miss, after hard pre-roll blockers. Fight Long heals20% of gross HP damage capped by pre-damage owner HP; it excludes overkill, ranged strikes and direct retaliation. Fractional healing is preserved; the whole-number rule governs damage. Shared-body owner identity must be retained even if a body is removed during a recursive effect.

## Complexity

A = current-floor actors; K = actor abilities; I = inventory; E = enhancements per item.

- Stat modifier predicates are O(K); stable owner lookup O(A). A shared endpoint's derived owner maximum additionally invokes normal effective stats, O(K + I×E), once; the owner branch does not recurse.
- Ready/consume and landed-hit mutations are O(A + K). They allocate no growing collection except a new condition entry when absent.
- Melee healing is O(A + K), plus Half Here synchronization O(A + copied endpoint metadata). It does not scan other floors or histories.
- Achievement check is O(K + H), H = achievement count, with a single log append on first unlock. Chronicle archival currently has its separately documented lookup cost.

No speedup claim follows from these bounds; full current-state/event/temporal integration remains a separate gate.
