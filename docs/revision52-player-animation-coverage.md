> Appearance override: the latest designer instruction replaces the historical decorative silhouettes and scene crossfades below with tile illumination or moving canonical glyphs. Trigger/geometry/privacy coverage remains applicable; old style-specific raster receipts do not certify the new look. See [Brogue-style contract](revision52-brogue-animation-style.md).

# Player animation coverage

Player hook implementation complete; cross-module integration and test execution are coordinated separately. This matrix identifies concrete emission boundaries, not exhaustive verification. All cues require revision 52, a detached recording batch, and observable cells/actors. Static entries intentionally do not produce map loops. Exact endpoints are independent marks, never interpolated teleport paths.

Automated tests in animation_player.rs passed in verification/r52-animations/core-focused-13.log. Direct positive/negative assertions cover P01/P02/P04/P07/P16/P37/P38, on/off/no-upkeep for every TOGGLES entry (including P21), unchanged movement P13, old-revision silence, static P14/P36/P49/P64, and newly-visible actor suppression. Other rows are code-inspected, not individually gameplay-tested.

| ID | Approved entry | Concrete trigger / static treatment |
|---|---|---|
| P01 | Rage at the Dying of the Light | divine_rescues::try_rescue, paid 1-HP rescue |
| P02 | Execute | revision50_combat::lethal_hit after eligible paid hit |
| P03 | Grappler | revision50_grappler::finish actual displaced endpoints; changed toggle bracket |
| P04 | Fight Long | divine_combat::heal_melee_damage_from: actual positive heal pulse plus directly observed victim-to-healer symbolic bead; unknown victim retains pulse only |
| P05 | Unstoppable Force | revision50_combat::after_attack: oriented wave-advance/speed-push vs opposing arc-swap-forward/reverse; dead target has no displacement cue |
| P06 | Masochism | revision50_combat::masochism_after_hp_loss once after crossing |
| P07 | You Shall Never Be Rid of Me | divine_rescues::try_rescue full-health rescue |
| P08 | I Came For You | Snapshot cameTarget change brackets observable target; engine closer movement credit |
| P09 | Fated Duel | Snapshot new duel activation, exact fixed 5x5 perimeter |
| P10 | Never Surrender | divine_reactions::pay_surrender successful once-turn payment |
| P11 | Bulwark of Mind | successful placement reveals the ward glyph first, then a nonblocking green nova centered on the ward; its visible glyph continues to pulse |
| P12 | Relentless Pursuit | Snapshot selected target changed, authoritative pursuit route; bonus hook |
| P13 | Hanged | engine::apply_hp_loss shield cue only for actual main-energy redirection; no stale toggle or retreat receipt |
| P14 | Tools of the Trade | inventory-only free potion/heart action (integration owner) |
| P15 | Make Slippery | Snapshot toggle; engine paid movement prevention |
| P16 | Quick Hand | divine_reactions::quick_hand_after_hit consumed reaction, including zero damage |
| P17 | Are You Sure About That? | divine_reactions::select_deflection_target: consumed reaction emits arc-redirect with both directly observed endpoints; ordinary redirected strike retains hit/miss |
| P18 | Spellslinger | inventory-only free amulet activation (integration owner) |
| P19 | Implant Thought | revision50::implant_thought: admitted stitch-thought player-to-attacker and notch-target attacker-to-victim, then ordinary actual strike |
| P20 | Distract | Snapshot distracted application; engine consumption |
| P21 | Mind Read | Snapshot Mind Read changed toggle only |
| P22 | Mind Slave | abilities successful Mind Slave at actual target |
| P23 | Weave of Time | engine exact reverse time-weave edge hook |
| P24 | Remember | abilities: one fracture-trait payload per actual newly acquired passive ID from observable source to player, followed by apparent target fold-outline; empty copy silent; copied passives retain own triggers |
| P25 | Phase Through | Snapshot Phase Through changed toggle; engine wall crossing |
| P26 | Rushing Torrent | Snapshot Rushing Torrent changed toggle; engine qualifying movement |
| P27 | Drown | engine actual water entry/healing/impact hooks |
| P28 | Float Away | engine Float Away actual retreat; Snapshot toggle |
| P29 | Half Here | half_here create/dismiss actual clone, no death |
| P30 | Riptide | divine_environment::apply_riptide actual steps; Snapshot toggle |
| P31 | Waking Nightmares | revision50_nightmares create_for/strike materialization and dissolution |
| P32 | Sudden Sleep | abilities paid Sudden Sleep |
| P33 | Dream of Time Gone By | Snapshot actual anchor toggle; abilities successful return |
| P34 | Sleep Walk | Snapshot energy reaches zero; engine qualifying movement |
| P35 | Dream-See | Snapshot Dream-See changed toggle only |
| P36 | Oblivious | STATIC: no map cue for hidden prevented actions |
| P37 | Overflowing | divine_rescues actual blood gain/rescue spend |
| P38 | Bleed Them | divine_reactions::apply_bleeding positive-damage live target |
| P39 | Blood Letting | abilities Blood Letting after damage/rescue and cleansing |
| P40 | Hardened Blood | Snapshot actual missing-health protection band change |
| P41 | Blood Weapon | Snapshot prepared mark; divine_combat consumed weapon |
| P42 | The Number of Blood | engine actual fifth-round attack consumption |
| P43 | Step Through Shadows | engine shadow haste transition/bonus movement |
| P44 | Silent Strike | engine qualifying silent impact |
| P45 | Unspeak | Snapshot Unspeak toggle only; suppressed casts stay quiet |
| P46 | Reverse Entropy | Snapshot Reverse Entropy activation exact observed portal-aware range; no upkeep/suppression pulses |
| P47 | Shadow in the Dark | Snapshot Shadow in the Dark toggle only |
| P48 | Not There | Snapshot Not There toggle only, no fictitious projectile |
| P49 | Boundless Hunger | STATIC: uncapped ordinary resource display |
| P50 | Entropy | Snapshot Entropy suppression/resume toggle only |
| P51 | Erase Metaphysical | engine successful trait removal; Snapshot toggle |
| P52 | Death Swift | engine haste after actual kill |
| P53 | Black Pit | divine_environment::leave_black_pit changed departure terrain; restore integration |
| P54 | Witch-Hatred | revision50_combat::lethal_hit authored Magickal Caster predicate |
| P55 | Shine | Snapshot Shine emission toggle only |
| P56 | Ascend | abilities actual successful Ascend destination |
| P57 | Nova | engine real Nova footprint; Snapshot toggle |
| P58 | Sparkle | abilities Sparkle real target, water puff |
| P59 | Star Power | Snapshot Star Power suppression/resume toggle only |
| P60 | Commandment | revision50_combat::after_attack groups the visible paid push steps into one actor-only slide |
| P61 | Cogworks | revision50_terrain admitted construction/removal exact footprint |
| P62 | Mini-Me | mini_me::commit/dismiss actual clone assembly/disassembly |
| P63 | Clay Army | revision50_terrain::activate_clay committed departure footprint; normal actual strikes |
| P64 | Appropriate | STATIC: inventory capacities and selected amulet costs |
| P65 | Tinker | inventory actual Tinker result (integration owner) |
| P66 | Bottomless Pack | inventory actual potion remainder (integration owner) |
| P67 | Fortune Teller | Snapshot actual fortune knot; engine consumption |
| P68 | Bonded By Unseen Threads | Snapshot bond toggle; engine actual bond damage |
| P69 | Turning the Wheel | Snapshot wheel toggle only |
| P70 | Sever Fate | abilities successful neutralization at actual target |
| P71 | See The Future | Current Unseen floor rewind: purple fade with scene swap at full opacity; prior revisions retain the legacy cue |
| P72 | All-Seeing Eyes | abilities paid All-Seeing Eyes |
| P73 | Dreamwalk / Wake Up | Snapshot actual Dreamwalk/wake state change |
| P74 | Teleport from the Masque of the Trickster (Ahvamitt’s reward) | ahvamitt actual Masque teleport (enemy/integration owner) |

## Fidelity follow-up

P04/P05/P17/P19/P24 now encode two independently observed symbolic endpoints, never an intermediate physical route. Seven new tests under animation_fidelity_tests / force_animation_fidelity_tests cover actual healing and identical Run state, full HP/hidden-source/legacy silence, distinct actual push/swap geometry, dead-target displacement suppression, reaction consumption and empty reactions, and real-new versus duplicate copied-passive payloads with apparent glyph outline. These seven selected tests passed in verification/r52-animations/core-focused-13.log. P19 has an additional admitted thought/notch endpoint assertion and invalid-target silence check.

## Final bounded style corrections

Real P01/P07 rescues now use composite collapse-snap/fold-reopen; P37 blood gain versus rescue uses droplet-fill/droplet-empty; P38 application uses droplet-two. P40 computes the actual Hardened Blood protection band from effective stats at both snapshot boundaries, choosing shield-harden or shield-soften. P67 uses stitch-bright/stitch-inverted. P46 emits only the four-neighbor boundary of the directly observed exact portal-aware radius mask. P68 activation links the actual observable bondedTarget with stitch-bond, otherwise remains local. P10 pay_surrender_from supplies an optional actual attacker to the shared facing helper only after successful once-per-turn payment; the existing API is preserved.

Five final_style_tests cover these style, delta, boundary and knowledge cases and passed in verification/r52-animations/core-focused-13.log. This does not imply a per-entry raster pass.
