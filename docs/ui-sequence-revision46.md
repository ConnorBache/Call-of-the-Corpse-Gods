# Revision 46 UI sequence: 2 → 3 → 4 → 8

This presentation-only variant starts from the exact Chronicle recovery source.
The mechanics revision remains 46; use `append-recording-trial` to retain the
experimental `corpse-gods-append-trial` save namespace. Core source, Cargo.lock,
toolchain pin, original art and the GPU/software fallback are preserved.

## Implemented in the requested order

2. Reserve a left column below Chronicle instead of relying on an incidental
map margin. The map remains right-aligned beside the existing right-edge HUD.
Every admitted visible/sensed creature has a measured, wrapped preview with
name, behavior, distance, Health, Energy and every condition/duration. Overflow
scrolls by pixels, including a single card taller than the column. Hidden actors
are not queried. Shared resources, invulnerability and infinite Energy retain
their existing meaning.

3. Draw an antialiased X over dead-creature glyphs. Living heartless creatures
retain their ring. A living sleeping body does not gain a corpse X. The flag
comes from the original actor's observed death state, before cosmetic identity
projection, and is carried through the existing rendering caches.

4. Blood on ordinary floor punctuation colors the dots rather than filling the
background. The same observed dry and bloody floor cell keeps identical
background bytes. Bloody walls, blood-contaminated water and Ichor remain
separate. Stored custom blood background/glyph settings no longer fill or
replace the floor punctuation; foreground appearance remains configurable.

8. Dialog sheets use 88% of the former linear size (77.44% area), with matching
native input/damage transforms. Their existing content and scroll ranges are
preserved. Pack has a 760×500 maximum with its existing separate content scrolls.
The gameplay canvas, main menu and nonmodal hover tooltip keep their scale.

## Verification commands

From the source directory, with the pinned Rust 1.99.1 and SDL2 available:

```sh
sh tools/cargo.sh test --workspace --locked --features append-recording-trial
ENEMY_LIST_SCREENSHOT=/absolute/previews.png sh tools/cargo.sh test --locked -p corpse-desktop --features append-recording-trial previews_preserve_special_resources_and_world
CORPSE_COMPACT_DIALOG_OUTPUT=/absolute/dialogs sh tools/cargo.sh test --locked -p corpse-desktop --features append-recording-trial compact_dialog
SDL_VIDEODRIVER=offscreen CORPSE_GPU_OUTPUT=/absolute/gpu sh tools/cargo.sh test --locked -p corpse-desktop --features append-recording-trial gpu_sdl_gameplay_visual_matrix -- --ignored --nocapture --test-threads=1
```

The native GPU matrix now includes corpse-X and actual blood-dot scenes as well
as gameplay, Pack, hover, styled text, Chronicle and animated terrain. Software
OpenGL is functional evidence, not physical-GPU verification. Cross-compiling a
Windows executable is not a Windows launch test.

See the recovery package's current verification report for the actual passed,
failed and blocked gates. Source availability or newly added tests alone is not
proof that compilation, screenshots or Windows runtime were verified.
