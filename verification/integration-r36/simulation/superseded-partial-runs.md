# Superseded development checks

These are development history, not current passing evidence. Failures were not removed from the final gate or silently counted as passes.

- Initial cargo checks reported E0282 for an untyped projectile pass closure. The closure was annotated with `movement::Point`; later checks compiled.
- One attempted cargo command named nonexistent target `portals_r35` and exited101 before running tests. Correct existing portal targets were subsequently used.
- Session74835 (11 physics tests): 9 passed /2 failed. NE aim ranking mistook adjacent occupancy aliases for zero-cost physical travel; ranking was changed to exact emitted physical distance. The NPC follow-up assertion incorrectly required a leftward coordinate step although a rightward stochastic-entry step was the valid route; it was replaced with actual repeated movement and target-arrival verification.
- Session57845 (11 physics tests): 9 passed /2 failed after alternate projectile origins were added. The directed physical adjacent-source range remained1, but an aimed projectile could legitimately use an alternate origin at range0; the test now checks explicit physical-ray emission. A reverse-origin fixture crossed another linked square after returning, so the expected coordinate ignored a real subsequent transfer; the isolation fixture was changed to an L-shaped group and now verifies exact translated direction, range and one RNG draw for both outcomes.
- A subsequent12-test physics run passed before possible-shot NPC admission was added.
- Session30806 passed15 physics tests plus the four named legacy reference-trace targets, before final actual-branch range guards and water-jet retargeting.
- Session28677 passed the final17-test physics target after those known fixes.

The lead's combined focused run and final frozen workspace gate supersede these partial development runs. No workspace-wide or Windows-runtime pass is asserted by this directory.
