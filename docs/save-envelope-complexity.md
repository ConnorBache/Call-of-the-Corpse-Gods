# Desktop save envelope optimization

`session_store::encode` still requests `Engine::canonical_run`, which exports the same legacy inverse-history representation as `Engine::save`. It then serializes a typed `SaveEnvelope<Run>` rather than serializing the Run to a string, parsing that string into a JSON tree, and serializing the tree again.

For B emitted envelope bytes, the desktop wrapper is O(B) time and O(B) output memory, **plus** the full time and memory of canonical history export. The prior wrapper additionally allocated an intermediate Run JSON string and JSON value tree and traversed both. This is a constant-factor/allocation improvement, not a change in the asymptotic export bound. Nested serialization, strings, escaping, and embedded history strings remain charged to B. The exported Run itself remains alive until serialization completes.

The envelope stays version 1, with the same field order and names. Decode still admits old version-1 envelopes and plain compatible Run saves. No save cadence, autosave key, recovery check, terminal archive/deletion rule, temporary-file rule, sync, or rename behavior is changed. Disk sync latency is external and is not bounded by CPU O(B).

Tests compare the new encoding byte-for-byte against the previous encode algorithm for 0, 1, and 5 retained frames, all session kinds, and floating-point edge spellings. Existing storage tests exercise changed saves, same-path corruption, restored-file retry, alternate paths, session isolation, and terminal behavior. An additional test covers unchanged-key no-write behavior, a preexisting temporary file, preserved destination bytes, and successful forced retry. These tests are not a claim about every possible Run or Windows runtime filesystem behavior.
