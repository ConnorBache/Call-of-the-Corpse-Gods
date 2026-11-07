# Revision 50 Grappler complexity

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

For A current-floor actors and K captured adjacent enemies, capture scans A actors and stores K identities and coordinates. Eligibility includes the existing portal-distance and dream-plane helpers, whose portal/cache costs are inherited rather than assumed constant. On each committed player step, destination validation uses existing passability, plane, portal, door, and diagonal checks. The collision planner sorts K candidates and scans A occupants for each: O(K log K + K*A) time and O(A+K) extra memory. K is at most A; ordinary non-overlapping local cells limit K to eight, but portal adjacency is not assumed to obey this bound.

Execution calls the existing forced-movement path up to K times, including terrain, travel, door settlement, actor lookup and other movement hooks. Their existing costs are additional to the planner; this entry does not claim constant-time movement. Blocked candidates remain in occupancy and block dependent candidates. Runtime checks handle changes caused by prior movement hooks. No gameplay randomness is introduced by the plan; forced-movement effects retain their existing serialized RNG behavior.

These are source-based estimates, not measured timings. Deterministic unit cases cover dependency order, blocked chains, outside occupancy, eligibility, condition restoration, and corner collision. Run the pinned native workspace suite and profile dense actor/portal floors to verify integration and costs. Native tests have not been executed in this editing environment.
