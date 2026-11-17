# Blood Mist
Overflowing spends one blood to survive lethal damage and emits Blood Mist.
The mist does not itself damage, inflict conditions, block movement, or alter
sight. It uses the existing single-gas-per-cell diffusion/competition rules.
Each use adds 100 volume (provisional tuning), rather than multiplying by the
remaining blood bank. Repeat uses can enlarge the cloud. Emission never erases
another gas directly: if its origin is occupied, it distributes its dose among
adjacent passable empty/mist cells; if all are occupied it emits nothing.
Mist survives up to five world/environment updates, tracked on every diffused
cell and saved/restored with the run. Mixed new/old mist retains the longer
remaining lifetime. No actor-haste or animation clocks are used.
It retains the existing general gas flammability rule (steam is the existing
exception): flame consumes it into ordinary gas fire, never explosive-gas
detonation. This is not an intrinsic status/damage effect of the mist.
