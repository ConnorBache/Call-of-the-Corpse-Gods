# Native gameplay acknowledgements (2026-11-17)

All acknowledgement pauses accept Space. They do not use Escape.

| Trigger | Display / result |
| --- | --- |
| Damage crosses from above 40% HP to 40% or below | `Health at or below 40%` and `Press space to continue`. |
| Damage crosses from above 25% HP to 25% or below | Equivalent 25% warning. Crossing both in one damage update shows only 25%. Healing above a threshold rearms it. |
| Paralysis, while playing and not choosing a blessing | `You are paralyzed. Press space to wait until un-paralyzed`. Space begins ordinary Wait commands until paralysis ends; health/death pauses or choices interrupt. |
| Knowingly walking into a visible chasm, or a known movement hazard leading to a chasm | `Press space to continue`, before issuing the queued movement. |
| Descend or Ascend while on a chasm | Same pre-action acknowledgement. |
| Other downward floor transitions, including forced falls / descent effects | Same prompt, holding the prior presentation; Space reveals the already-resolved outcome without issuing an extra turn. Ordinary Descend is excluded. |
| Death | Space advances from the death scene to the run summary. |
| Other terminal game scene (successful run) | A fresh Space press after presentation now opens the run summary; existing movement-key/click controls are retained. |

Health, paralysis/chasm, and terminal-run gates require a fresh Space press after presentation, rejecting repeat/queued presses. Animation input locks resume automatically when presentation ends and cannot be skipped with Space.

## Not acknowledgement pauses

- Hazard confirmations for fire, razorleaves and harmful gas, including known portal/current/Non-Euclidean arrival hazards: Y confirms; N or Escape cancels. Space does not consent.
- See the Future's energy-spending and saved-state-return confirmations: Y confirms; N or Escape cancels.
- Blessing selection: choose the desired option using numbers or selection plus Enter. Space does not choose an ability.
- Inventory, inspect, help, bestiary, chronicle, settings and main menu are interactive screens, not forced acknowledgements; their own controls are retained.
- Rest/travel/autoexplore interruptions return control to the player rather than holding a dismissible modal. Replay playback is separate: Space steps, P toggles playback.

This audit checks native source paths and focused tests, not a packaged Windows executable. No simulation state, turn scheduling or replay schema changes are introduced.
