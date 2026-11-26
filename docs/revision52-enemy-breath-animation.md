# R52 enemy breath presentation

A successful paid Pale Gasser or Greater Gasser exhalation draws a brief,
one-cell-radius outward wave from the caster. The wave has no gameplay or
gas radius: the cast still deposits gas only at its source, and the ordinary
environment controls diffusion. Passive emission is not a repeated scream.

Dragon and piloted Mech breath keep their authored cone, damage, ignition,
collision, and portal/pocket rules. The committed, directly visible
footprint travels in parallel distance bands using Firebolt's light trail
and its 16 ms step clock. The painted fire appears on a reached tile as
the leading edge moves on. Hidden or newly disclosed terrain is not
inserted into the pre-cast scene. The animation does not rerun any part of
the simulation or control when damage and fire apply.

## Complexity

Let C be the real cone footprint, O the cost of a visibility query, P the cost
of a topology-aware distance query, and V the number of detached view layers
plus their copied payload. On successful breath, receipt admission and ordered
serialization add O(C log C + C(O + P)) work and O(C) storage; there are at
most six travel bands. The native compositor creates at most six observation
views for a displayed cast, using the existing COW action checkpoints. Each
constructed view costs O(V log(C+1)) time and O(V) memory, including its
per-tile layer lookup (view copying included). At draw time one cached view is
selected by the existing sequence and the magic travel pass emits O(C) light
samples before normal clipped native paint. Gas exhalation tests at most five
fixed footprint cells for visibility and does not copy an action outcome for
this effect. These are analytical bounds, not frame benchmarks.
