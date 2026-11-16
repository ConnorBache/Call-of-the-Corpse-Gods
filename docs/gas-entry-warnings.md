# Gas entry warnings

Designer instruction: when the player is currently in any gas, stepping into gas
must not prompt again. This covers the same or a different gas type, including a
harmless gas at the player's current position. It is a UI-only change, including
when viewing an older saved run; simulation revisions, gas effects, propagation,
turn costs, RNG, save payloads and time-travel history are unchanged.

The warning projection uses `environment::gas_present` at the player's current
cell in the active observation plane. Canonical gas-type presence counts as
occupancy, even at zero volume. With legacy channels, any positive channel counts.
No new concentration threshold is introduced. Destination gas remains gated by
current visibility; remembered or unknown live gas does not create an entry
warning. The check is recomputed per command, so clean-air entry warns again.

Keyboard moves, adjacent mouse moves, queued click-to-walk and auto-explore all
reach `hazards_ui::intentional`. Direct, confused, current-displaced, portal and
Non-Euclidean arrivals share the same gas-entry rule. Fire, razorleaves, chasm and
other independent warnings remain; ordinary waiting and current warnings while
waiting remain unchanged. Route hazard preferences, threat stops, damage stops,
incapacitation stops and movement validity checks are unchanged.

## Complexity

The additional source-cell query has O(1) time and O(1) extra memory per exposure
candidate, using canonical indexed gas presence or the fixed nine-kind legacy
catalog. It performs no visibility search, allocation, world mutation or cache
work. It is evaluated only while projecting an input/travel warning, not per
rendered frame. Existing destination observation and warning string allocation
costs are unchanged: `observed_hazards_at_revision` builds its environmental
observation context, and visible-target tests retain their existing vision-query
cost. Confused movement and linked arrivals retain their existing candidate-loop
bounds. This is a cost analysis, not a measured timing claim.

## Regression coverage

`hazards_ui::gas_entry_tests` covers all nine source gases against the five warned
target gases; clean-air reentry; normal/haste input phases; canonical and legacy
occupancy; unchanged waiting; fire/razorleaves/chasm coexistence; confused input;
hidden and remembered gas; currents, portals, Non-Euclidean exits and dream-plane
observation; shared keyboard/mouse/queued route commands; and auto-explore's
independent fire and threat stops. Warning calls are checked for state purity.
These are deterministic unit/controller checks, not an OS-level interaction claim.
