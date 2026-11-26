# Stone Seal expiry and enemy cast receipt audit

Stone Seal keeps its original placement eligibility, 15-energy cost, three-turn
shared cooldown, and five-normal-world-turn wall duration. The casting floor
and plane now own each wall's expiry, so a move, fall, teleport, or death of
the caster cannot move the expiry onto a different floor or strand the wall.
Floor ownership restores the ordinary floor tile only if the seal is still a
wall. While booked, no other temporary or persistent wall may use the same
tile and lose its independent restoration receipt. Shattering and permanent
collapse cancel the lease when destroying the wall, before any replacement.
Fire only converts grass and trees, not Stone Seal walls. A newly created dream layer
does not copy the waking Stone Seal.

The recorded command policy `stoneSealFrom` starts at zero in new R52 runs.
Historical replay commands and journal tails use the old actor-owned form until
the adoption boundary. In-flight legacy casts have no origin-floor evidence.
After adoption their actor cooldown settles normally, but their wall can remain:
the legacy timer never guesses which map to edit or erases an unrelated wall.
New receipts persist in floor snapshots
and follow terrain through save/load and time restoration. An adjacent floor
may expire its own seal in its normal simulation. A floor outside the existing
local horizon is never advanced just to expire a wall; expiry is settled when
that floor next receives its normal simulation.

The normal-turn hook inspects only the current simulated depth and at most two
planes. For F stored floors and S receipts on that depth, the floor lookup/due
test costs O(F + S) time and no extra array if nothing is due; applying a due
batch scans in insertion order in O(S) time and uses O(S) temporary storage.
Each actual removed wall also pays one existing liquid frontier update and
visibility/event admission. The seal vector is bounded by the number of distinct
cells on each floor. Overlap checks add O(S) reads per candidate tile, with at
most eight Stone Seal candidates and bounded footprints for Cogworks and other
temporary effects. Save validation inspects all floor headers plus O(total
receipts) to reject duplicate, nonfinite, or out-of-bounds records. No new RNG
draws or autonomous traversal are added.

A Dryad's paid healing cast may miss when a non-Euclidean fork diverts the
spell. The Chronicle now acknowledges that paid cast, without claiming it
healed anyone or adding a duplicate on a successful heal. This does not change
the RNG branch, selected ally, health, energy, cooldown, or recovery.

## Enemy audit scope

The source audit followed the enemy trait catalog into actual simulation paths,
including the intrinsic/legacy-name normalization before routing. The player
ability audit is separate. This checks for silent/unreachable dispatch or a
reported action that never commits; it does not rebalance AI, require every
observable cue to be a successful hit, or claim to playtest every generated
dungeon and status combination.

| Enemy group | Action and effect paths reviewed |
|---|---|
| Slimes, ordinary wildlife, beetles and obelisk creatures | Default action scheduling, Fast/Slow and flight; Blorb and Killspawn melee follow-ups, Glim habitat/trail, Tiger/Sabretooth stalking, Bombardier detonation, Maulchild hold. Unpowered Rat, Bear, Dire Bear and ordinary bat/beetle variants use the shared paths. |
| Saidribees and Needleling | Three venom strengths and ticks, Guardian grab, Queen summons and short/guardian offspring. |
| Horn family | Odd-horn ranged spell routing and recovery, Beckon, Stone Seal placement and expiry; even horns use ordinary Fast movement/melee. |
| Razorwood and mushrooms | Dryad ally ranking, paid cast/hit/visibility; Samara razorleaves; Corpse Fungus roots, corpse retention and gas. |
| River and Ichor forms | Kelpie deep-water grasp, River Drake submerge/hold/jet, all Lesser/normal/Greater fish/shark/leviathan variants share Ichor habitat and siphon, Mercurial revival conversion. |
| Moon family | Moonspawn Blink, Valta charge/release and fire traits; the dedicated Valta caster's lack of approach Blink is intentionally documented. |
| Hemogoblins and breath casters | Ordinary combat, Bloodmancy phases, Inventor retreat/mount, dormant/piloted Mech, Dragon takeoff/landing, and shared Mech/Dragon/Chimera fire breath. |
| Undead, mythic, support, guardian and late wildlife | Vampire retreat/lifesteal, Wendigo feeding, Banshee clock and witnesses, Witch summon/curse, Night Crawler lifesteal, Skin Walker disguise, Troll regeneration, Yeti chill, Phoenix rebirth, Cancer armor bypass, Ursa death cry, Minotaur committed bump, Sphinx provocation/hunt. |
| Pale roles and offspring | Jackal and Chomper follow-ups; regular/Greater Gasser emission/hit, Spitter projectile/secondary gas/recovery, Birther/twins, Screamer status/rally and lockout. |
| Godbeast encounters and finale | Zillennia stages share Ruin Pulse, silence, darkness and tier-gated tile erasure; Mercuria activation/flood/birth, Verdania harvest/guardian, Borge demand/offering/reward, Phantasmii realm/ritual, Epidite blessing, Ahvamitt progression, Administrator death/fall/collapse. |

Two documented behaviors were left unchanged: Valta presently forfeits a
charged shot if a clear target becomes out of range or untargetable (it retains
the charge if the path is blocked); fire cones require a guaranteed direction
through ambiguous non-Euclidean forks, unlike an aimed bolt that can commit a
random branch. Changing those would change combat rules.

A baseline core-unit run before this patch passed 984, failed 15, and skipped
8; the failures were in older progression, animation and amulet/ring/gas/potion
coverage. The 96 focused enemy baseline tests passed (1 ignored). Fresh
regressions in `enemy_cast_receipts` cover the paid Dryad miss and live Stone
Seal paths, old in-flight handling, replay boundary, both planes, and shattering.
