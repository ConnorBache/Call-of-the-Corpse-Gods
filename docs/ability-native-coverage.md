# Native ability execution coverage

As of the current integration checkpoint, **all 72 divine ability names have admitted native executable paths** (including Revert/Future through the temporal Engine wrapper). This is not complete-game coverage: Remember admits all 44 authored enemy passive traits and supported divine passives; Mini-Me admits all 15 authored starter powers or an empty ability list. Unsupported copied powers fail before payment or mutation. NPC divine scheduling remains restricted. Cross-ability combinations beyond the named fixtures remain unproved.

Catalog frozen from v48. These are native statuses, not the reference game’s support status. Unlisted effect hooks are rejected before committing a command. All definitions can be offered by progression; selecting unsupported mechanics remains rejected.

| Ability | Native scope | Evidence |
|---|---|---|
| Rage at the Dying of the Light (shrail-rage-at-the-dying-of-the-light) | Executable: authored toggle/passive, acquisition-order rescue, retained costs and counters | rescue_commands:6scenes60 full command/event steps, zero/five energy, repeated damage |
| Unyielding Assault (shrail-unyielding-assault) | Starter toggle; combat owner handles paid double sequence and offhand gate | Inventory dispatch toggle trace; attack coverage belongs to combat |
| I Hit Back (shrail-i-hit-back) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Fight Long (shrail-fight-long) | Executable: post-melee healing from actual damage | divine_combat source vectors; blood_weapon_commands:12scenes56 combined command/event steps |
| Beat to Death (shrail-beat-to-death) | Executable: landed/live-target dodge and DR debuff, including zero damage | divine_combat source vectors; blood_weapon_commands:12scenes56 combined command/event steps |
| Fury Crafting (shrail-fury-crafting) | Executable: authored craft/toggle and once-per-attack ordered funding | fury_commands:9scenes48commands, funding thresholds and Unyielding Assault |
| You Shall Never Be Rid of Me (pelian-you-shall-never-be-rid-of-me) | Executable: authored toggle/passive, acquisition-order rescue, retained costs and counters | rescue_commands:6scenes60 full command/event steps, zero/five energy, repeated damage |
| I Came For You (pelian-i-came-for-you) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Never Without a Fight (pelian-never-without-a-fight) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Never Surrender (pelian-never-surrender) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Bulwark of Mind (pelian-bulwark-of-mind) | Free 20-energy ward placement, enemy-entry guard and end-turn expiry; full command source trace passed | See subsystem tests; no blanket complete-game claim |
| Zeal Craft (pelian-zeal-craft) | Equipped craft stats and free 15-energy 25%-HP healing; full command source trace passed | See subsystem tests; no blanket complete-game claim |
| Missed Me (isheilah-missed-me) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Tools of the Trade (isheilah-tools-of-the-trade) | Executable: free heart/potion use, source Fresh suppression | consumption_passives: Tools/Hunger combinations; 44 total commands |
| Make Slippery (isheilah-make-slippery) | Executable: target toggle and8-energy movement denial before collision | shine_slippery:8scenes48 command/state/event comparisons across funding boundaries |
| Quick Hand (isheilah-quick-hand) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Are You Sure About That? (isheilah-are-you-sure-about-that) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Guile Craft (isheilah-guile-craft) | Equipped craft stats and +5 capacity per enhancement, removal blocker; full command source trace passed | See subsystem tests; no blanket complete-game claim |
| Implant Thought (numo-implant-thought) | Native targeted execution and ally expiry; 93 command/event comparisons passed | Thresholds, sleep, self targets, temporary expiry |
| Distract (numo-distract) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Mind Read (numo-mind-read) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Mind Slave (numo-mind-slave) | Native targeted execution and ally expiry; 93 command/event comparisons passed | Thresholds, sleep, self targets, temporary expiry |
| Revert (numo-revert) | Temporal Engine execution verified by 64 command/event comparisons, reconstructed history snapshots, free actions, haste, retained costs and save/resume; raw core activation deliberately rejected | See subsystem tests; no blanket complete-game claim |
| Remember (numo-remember) | Executable copy: all 44 authored enemy passives and supported divine passives; unknown metadata rejected before mutation | remember: 31 scenes / 280 commands plus unknown-target atomic guard; 28 divine copy vectors; remember_hazards: 8 scenes / 64 commands; all 44 expansion: 139 scenes / 1428 commands, plus frozen-metadata and unknown-target guards |
| Phase Through (etill-phase-through) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Rushing Torrent (etill-rushing-torrent) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Drown (etill-drown) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Float Away (etill-float-away) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Half Here (etill-half-here) | Executable: activation/dismissal, linked health/resources, endpoint-local statuses and upkeep exclusion | half_here_commands: 3 scenes/23 commands; lifecycle11 and fall/collapse6 fixtures; linked temporal/hazard suites being expanded |
| Riptide (etill-riptide) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Waking Nightmare (ursa-waking-nightmare) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Sudden Sleep (ursa-sudden-sleep) | Executable: paid 20-energy full healing and paralysis51 before world turn | expanded_consumables278 suite: dedicated Sleep command/state/event steps |
| Dream of Time Gone By (ursa-dream-of-time-gone-by) | Executable:50-energy anchor, free validated return, cross-floor vertical travel | dream_anchor:5scenes16commands, insufficient funds and blocked destinations |
| Sleep Walk (ursa-sleep-walk) | Executable: authored passive, source surprise or run-aware haste hook | stealth_passives: 12 scenes / 72 commands, zero/one energy and wandering/hunting targets |
| Dream-See (ursa-dream-see) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Dream Craft (ursa-dream-craft) | Executable: equipped shared-owner flight and rounded1.5maxEnergy, removal resource/fall order | dream_craft:10scenes30commands, water/fire/chasm/pit, with/without HalfHere |
| Overflowing (lichor-overflowing) | Executable: authored toggle/passive, acquisition-order rescue, retained costs and counters | rescue_commands:6scenes60 full command/event steps, zero/five energy, repeated damage |
| Bleed Them (lichor-bleed-them) | Executable: authored activation/passive, ordered damage/reaction hooks and counter initialization/refresh | reaction_commands:12scenes84command/event comparisons, zero/full energy and Nightmare recharge boundary; divine_reactions vectors |
| Blood Letting (lichor-blood-letting) | Executable: free tuned self-damage, authored status removal, resource/shared-body sync | blood_letting:6scenes18commands, lethal/rescued costs and buff/status removal |
| Hardened Blood (lichor-hardened-blood) | Executable: source missing-health DR calculation after derived maximum health | divine_combat source vectors; blood_weapon_commands:12scenes56 combined command/event steps |
| Blood Weapon (lichor-blood-weapon) | Executable: exact rounded HP cost, priming, guaranteed doubled attack, consume on hit/miss | blood_weapon_commands:12scenes56commands/events, repeat priming, lethal and maxHP rounding |
| The Number of Blood (lichor-the-number-of-blood) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Step Through Shadows (dawel-step-through-shadows) | Executable: authored passive, source surprise or run-aware haste hook | stealth_passives: 12 scenes / 72 commands, zero/one energy and wandering/hunting targets |
| Silent Strike (dawel-silent-strike) | Executable: authored passive, source surprise or run-aware haste hook | stealth_passives: 12 scenes / 72 commands, zero/one energy and wandering/hunting targets |
| Unspeak (dawel-unspeak) | Executable: exact intrinsic initialization, dynamic cost refresh, maintenance and silence scheduling | unspeak: 10 scenes / 50 commands, legacy and initialized Pale targets, funding boundaries |
| Reverse Entropy (dawel-reverse-entropy) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Shadow in the Dark (dawel-shadow-in-the-dark) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Not There (dawel-not-there) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Boundless Hunger (grisha-boundless-hunger) | Executable: uncapped energy gains, finite equipment-cap losses, reference meter | consumption_passives: Tools/Hunger combinations; 44 total commands |
| Entropy (grisha-entropy) | Executable: emission suppression toggle, caustic emission and immunity | divine_environment: full state/events including incoming caustic and zero-cost off; environment kernel fixtures |
| Erase Metaphysical (grisha-erase-metaphysical) | Executable: free toggle and authored combat/spell/upkeep hooks | final_toggles: 18 scenes / 108 commands, six energy boundaries; further interactions remain unproved |
| Death Swift (grisha-death-swift) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Black Pit (grisha-black-pit) | Executable: activation/upkeep and composed movement/environment/resource hooks | advanced_environment_commands:19scenes93 full command/event steps; phase crushing and funding boundaries; compound interactions expanding |
| Uncast (grisha-uncast) | Executable: free toggle and authored combat/spell/upkeep hooks | final_toggles: 18 scenes / 108 commands, six energy boundaries; further interactions remain unproved |
| Shine (anavani-shine) | Executable: light emission, whole-source suppression at revision46 (self-only through45), target-light accuracy | Legacy shine_slippery traces; revision46 suppression/overlap/cache/planes/visibility/accuracy/temporal command tests in shine_revision46 |
| Ascend (anavani-ascend) | Executable: passive flight and free 50-energy upward travel through chasm | ascend_commands:7scenes19 command/state/event comparisons, including water/fire flight contact |
| Nova (anavani-nova) | Executable: toggle,30-energy per hit/miss explosion at captured impact and passive blast immunity | nova_commands:8scenes44commands/events, melee/ranged,0/29/30/60energy; environment immunity vectors |
| Sparkle (anavani-illuminate) | Executable: free 20-energy visible combustible target, permanent brazier ignition | divine_environment: complete command state/events, invalid target and repeated cast; environment ignition fixtures |
| Star Power (anavani-star-power) | Executable: toggle, permanent burning, incoming fire immunity | divine_environment: full state/events, incoming fire and zero-cost off; environment kernel fixtures |
| Make Kneel (anavani-make-kneel) | Executable: free 20-energy targeted paralysis10 | expanded_consumables278 suite: dedicated Kneel command/state/event steps |
| Divine Craftsman (kative-divine-craftsman) | Two enhancement slots per item; full command source trace passed | See subsystem tests; no blanket complete-game claim |
| Mini-Me (kative-mini-me) | Executable bounded clone: actual Avatar initial starters or empty powers; other copied powers rejected atomically | mini_me_commands: 39 scenes / 813 commands; all 416 valid three-god starter combinations / 2080 commands; offscreen 32 scenes / 768 commands; root construction 48 vectors |
| Adaptive Armor (kative-adaptive-armor) | Executable: canonical heart-type counts, first three contributions, triangular mitigation capped at three | consumption_passives: acquisition/counts; combat owner mitigation vectors; broader species interactions remain untested |
| Artifice Craft (kative-artifice-craft) | 5× rounded heart bonuses on equipment, no consumable energy/blessings; full command source trace passed | See subsystem tests; no blanket complete-game claim |
| Tinker (kative-tinker) | Executable: exact two-draw reservation, naming, payment and equipped grant reconciliation; unsupported draws fail atomically | tinker_commands: all 47 eligible outside-god reserved grants / 188 commands, two-draw selection, naming, duplicate attempt and equip/unequip |
| Dilute Potion (kative-dilute-potion) | Executable: retained first potion and half-strength second use for admitted effects | expanded_consumables278; descent_potions8; all effects including incineration/gases/Sap in hazard_potions200 and thrown_potions414 |
| Fortune Teller (morae-fortune-teller) | Executable; relevant cases in starter_modes and actor_parity suites | Exact source state/events; combinations beyond fixtures remain unproven |
| Bonded By Unseen Threads (morae-bonded-by-unseen-threads) | Executable: initial payment, upkeep, targeted link and secondary resolved damage sharing | bonded_commands: 10 scenes / 50 commands across funding and lethal boundaries |
| Turning the Wheel (morae-turning-the-wheel) | Executable: free toggle and authored combat/spell/upkeep hooks | final_toggles: 18 scenes / 108 commands, six energy boundaries; further interactions remain unproved |
| Sever Fate (morae-sever-fate) | Native targeted execution and ally expiry; 93 command/event comparisons passed | Thresholds, sleep, self targets, temporary expiry |
| See The Future (morae-see-the-future) | Temporal Engine execution verified by 64 command/event comparisons, reconstructed history snapshots, free actions, haste, retained costs and save/resume; raw core activation deliberately rejected | See subsystem tests; no blanket complete-game claim |
| All-Seeing Eyes (morae-all-seeing-eyes) | Free 30-energy floor reveal, expiry after completed-action observation; full command source trace passed | See subsystem tests; no blanket complete-game claim |

Complexity and known subsystem gaps: [abilities-inventory-complexity.md](abilities-inventory-complexity.md).

Equipment-granted admitted abilities now execute with stable ownership, dormant counter retention and linked cleanup. Nine source command scenes (48 steps) verify resource/flight/death ordering and learned promotion; the separate equipment_abilities suite covers192 projections/load reconciliations. Unsupported granted mechanics still reject explicitly. Five raw combat scenes additionally compare full state/events for lethal/terminal I Hit Back and offhand counter/deflection; these do not lift NPC divine-ability scheduler admission.

## Copied-power boundaries

Remember now admits the immutable catalog's 44 enemy passive traits. Source differential scenes cover copying, combat, ordinary movement, expiry and Half Here for every trait; specialized NPC-only AI remains inert on the player where the source does so. The final all-trait rerun passes after exact habitat-stranding, copied-Mech identity and retained-endpoint gas provenance fixes. Mini-Me admits all 15 frozen authored starter powers or no powers. Every 416 valid three-god starter combination passes, as do offscreen clean/precise/lethal cohorts. Nonstarter initial-player powers remain rejected before payment; arbitrary saved initial-player profiles and general NPC divine activation/scheduling remain execution restrictions.

## Revision 51 confirmed corrections

[Revision51 ability restorations](revision51-ability-restorations.md) supersedes
older table prose for I Came For You, Sparkle, All-Seeing Eyes and Shine. Stable
ability IDs and command admission remain supported. R51 execution and deterministic
coverage are described there, including the unresolved Ichor-spread distinction;
older revision assertions and historical verification claims remain separate.
