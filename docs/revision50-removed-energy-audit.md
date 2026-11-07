# R50 removed energy costs audit (2026-11-07)

## Resolved by explicit designer instruction

“Give all those energy costs back except for Implant Thought.” The four amounts
below are restored in the R50 catalog and simulation: Dream-See 1/turn, Mind Read
1/turn, Bonded 15 activation plus 8/turn, I Came For You 10 activation. Immediate
maintenance is prepaid, so Bonded initially pays 23. Implant Thought remains 15.
The remainder records the pre-correction audit, not current unpaid behavior.

## Original findings

Compared all retained ability IDs in `crates/corpse-core/catalog/abilities-v48.json`
and `abilities-v50.json`, then checked native revision branches and desktop costs.
Four retained abilities changed from paid to zero energy:

- **Dream-See:** old active description specified 1 energy per turn. R50 request
  section Ursa/5 and catalog omit that cost. `abilities::upkeep_cost_in` explicitly
  returns zero in R50, despite the legacy `upkeep_cost` value of 1. Passive sleeping
  telepathy remains free. Latest designer instruction requires the activated
  version to cost energy; the numerical amount remains pending clarification.
- **Mind Read:** old active description specified 1 per turn. R50 request Numo/3
  and catalog omit the cost; the same explicit native override returns zero.
  Passive radius-5 telepathy and active radius-20 telepathy currently cost zero.
- **Bonded By Unseen Threads:** old description specified 15 on activation and
  8 per turn. R50 request Morae/2 and catalog omit both. Native activation wraps
  the 15 payment in `!revision50::enabled`, and upkeep overrides 8 to zero.
  Its target-selection/damage-link mode currently costs zero.
- **I Came For You:** old description specified 10 on activation. R50 request
  Pelian/2 and catalog omit it. Native target activation explicitly skips the
  10 payment when revision 50 and this ID match. Its telepathy/movement benefit
  currently costs zero. Its duration also changed in R50, independently of cost.

These are explicit implementations of the R50 rewritten descriptions, not losses
caused by reserve accounting. Omission alone does not prove that the designer
intended them to become free. No new amount is inferred by this audit.

Desktop `ability_ui.rs` mirrors the native changes: R50 activation branches make
I Came For You and Bonded zero; upkeep comes from revision-aware native upkeep.
Thus the four free labels agree with current simulation, but Dream-See's free
behavior no longer meets the newest designer instruction.

Other retained descriptions losing the word “energy” are **Half Here**, **Entropy**,
**Shine**, and **Star Power**. Those did not lose a positive cost: Half Here described
shared resource ownership, and the other three expressly described free toggles.
They must not be reported as newly free. Reaction/nightmare/blood resources are
not energy costs. Retired/replaced ability IDs are not retained abilities becoming
free. Price reductions such as Implant Thought's 30 to 15 remain paid and are not
part of this list. Existing attack/reaction costs are unchanged by the activation
payment correction.
