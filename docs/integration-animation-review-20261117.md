# Animation integration review, 2026-11-17

Independent integration pass; no release/push performed by this reviewer.

## Verified implementation boundaries

- Engine resolution remains atomic/deterministic; detached presentation checkpoints gate visible results and finite animation input locking. Forced-fall pauses hold the prior-floor scene after resolution, while intentional falls/Ascend are queued before dispatch.
- Pause holding now paints outgoing-floor animation cues, never later-floor snapshots. Routine damage/new threats do not terminate paralysis waiting; required pauses/death/end/explicit interruption do.
- Travel receipts within the same action serialize, including a magic cast followed by displacement. Added an exact same-stage timing regression.
- Banshee victims are resolved in stable Euclidean radius/actor-id order to match screen-space wave arrival. Gameplay sight admission remains portal-aware; visual radial propagation is screen-space, not portal-distance travel.
- Initial populated Verdania checks exposed both ordinary-budget category confusion and decorator relocation risk. The dedicated guard encounter category preserves ordinary budget accounting and guard placement. Broader matrix owned by the guard implementation reviewer.

## Historical desktop test corrections

These are expectation/fixture updates, not permission to weaken production behavior:

- Preserve Clay Army's one wall sentence even when a legitimate unavailable-resource line follows it.
- See the Future detail includes the user's approved standalone energy paragraph; immutable catalog/offer checks remain.
- Honor authorized Ragelight/Never Be Rid of Me aliases and Ragelight copy while retaining catalog immutability.
- Removed reserve-energy tutorial stays absent; reserve capacities/payment/immutability remain asserted.
- Shared clone detail omits the redundant independent-action sentence from historical v48 copy.
- Bow hover oracle hit percentages are rounded to integer display; exact damage/resource text remains checked.
- Visible terrain inspector omits user-rejected titles; bloody-wall description, map label, and remembered-information checks remain.
- Modal geometry accounts for current left-panel reservation and inspector strip; containment/profile checks remain.
- Pack appearance cases start with independent view fixtures rather than carrying a previous scroll position into a large-font reflow and clicking unscrolled coordinates.

## Test runs

Initial full core lib: 855 passed, 5 failed, 6 ignored. One known baseline authored progression-weight mismatch (Ragelight25 vs27); one newly strengthened Dryad visibility expectation subsequently fixed/passed; three Verdania ordinary-budget accounting tests corrected by guard owner. Initial desktop full run and final affected rebuild/rerun pending at time of this note; consult follow-up result log/report before release. Windows executable runtime not verified here.

### Weighted-choice baseline evidence

Compared against `Call-of-the-Corpse-Gods-integrated-candidate-source.zip`: `abilities-v48.json` is byte-identical (SHA256 `51cc7d859645d29f7632f144f9c9e7ef1c926ce621fd233710e9efc5f69df739`); `progression-v48.json` also byte-identical (`ab1ca1ca31862a72662ac035bfec50f9a86ff33216290204946cd5362cca4596`). The only archive/current `progression.rs` difference is level-up cost, not catalog weighting. With the failing fixture's15 owned abilities, Ragelight description yields energy/death tags, two same-god abilities,12 summed overlaps: `1 + 2*12 = 25`; fixture expects27. This proves the mismatch is present in archived source/fixture inputs, not introduced by this animation integration. Intended probability cannot be inferred safely from the mismatch; neither production weights nor frozen expected values were modified.

Additional targeted passes: all six divine animation shape/order/input tests, exact wave death/HP/status arrival against renderer lighting at325ms/650ms,1024-case creature description oracle, modal baseline, bloody-wall current/memory inspection, pack appearance fixture. Final future paragraph and bow oracle expectation test edits still await the final rebuild.

Final core full run:866 passed,1 failed,7 ignored (493.67s); sole failure is the archived weighted-choice mismatch above. Final targeted desktop passes include future paragraph (details/offers),141 bow forecasts,6 gameplay pauses,4 health warnings, wave-arrival regression,64 pack grouping/copy fixtures.

The broad desktop run also exposed a stale760px Pack-width cap; corrected to viewport containment to honor the wider inventory request. A separate black-glyph raster assertion remains unresolved: at1280px Dire Bear's darkest pixel isRGB(13,13,13), versus the test's <=3. Authored foreground remains black and outline remains190 gray. Archive/current test, mask helpers, fonts and atlas are identical; static glyph paint code is unchanged (render differences are animation hooks). This is source-comparison evidence, not a separately executed baseline. No tolerance or production raster behavior was changed. A separate baseline build was deferred to avoid duplicate multi-GB build pressure during delivery. Do not call the entire desktop suite green.

Broad desktop discovery finished on the initial binary:1074 passed,23 failed,40 ignored (1000.64s). The initial binary was intentionally retained during source corrections, so this is not a final-tree green run. Four Bestiary failures were fixed by the Bestiary reviewer and independently rerun. Three separate-process persistence failures were `ENOENT` when the running old binary's executable path was replaced during compilation; all three pass on the current binary. Other failures were the explicitly documented approved-copy/layout expectation drift, plus the unchanged unresolved black-ink raster assertion. Additional stale expectations: removed arrow tutorial, expanded Pack width, See the Future custom indicator, removed Ice/Puddle/Hellfire headings. Reduced/instant replay still presents finite static feedback, so it waits until those visible marks end, rather than pretending there is no presentation.

Final affected reruns complete: expanded Pack dialog containment, Future custom indicator, reduced/instant finite-feedback replay, and all6 sub-biome UI tests pass. The remembered Ice-title assertion is preserved: only current terrain titles are suppressed. All23 original desktop failures were classified;22 now pass targeted reruns (including4 Bestiary tests fixed by their owner and3 process recovery reruns), leaving only the unmodified native black-glyph pixel assertion. No full final-tree desktop rerun was performed after these fixes; no claim of entire-suite green. The final desktop test executable builds successfully with1139 tests including the two added sequencing regressions. No GitHub push or Windows-runtime verification performed by this reviewer.
