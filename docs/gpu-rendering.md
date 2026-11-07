# Accelerated gameplay drawing

The desktop now attempts SDL's accelerated renderer first. If it cannot create
one, it retries the same window with SDL's software renderer. The explicit
`--software-renderer` switch or `CORPSE_RENDERER=software` retains software-only
operation. No graphics card is required to play. An accelerated API can itself
be implemented by a software driver (for example Mesa llvmpipe); SDL's driver
flag is not proof of physical GPU execution or a performance improvement.

Gameplay and replay drawing record ordered native-resolution rectangles and
textured quads instead of blending the completed frame on the CPU. Existing
fontdue outlines and authored ASCII atlas masks are prepared on cache misses,
uploaded into GPU texture atlases once, and then tinted, rasterized and blended
by SDL's accelerated geometry path. Adjacent operations sharing an atlas batch
together without changing their order. The GPU applies the Chronicle fade as
native-pixel rows. No whole painted game framebuffer is uploaded in this path.

Simulation, visibility, observation, input dispatch, RNG, saves and replay
formats are unchanged. CPU layout, mask generation and cosmetic decisions
remain CPU work. GPU drawing does not move the game simulation to the GPU.
Native dimensions, clipping, source artwork and draw order are preserved.
GPU normalized-byte alpha blending can differ from the reference's integer
floor and floating-point rounding by a few channel values; exact RGBA identity
is not promised. Visual comparisons measure that difference separately.

The menu, illustrated bestiary, settings and other non-game screens currently
retain their existing CPU painter. Headless PNG exports and software-only mode
also retain the reference painter. The software framebuffer and fallback
streaming texture stay allocated so a drawing failure can recover promptly.
Rejected oversized resize requests use the existing software presentation path.
Texture/device reset invalidates GPU caches. A failed atlas allocation or GPU
submission drops GPU resources, recreates an SDL software renderer for the same
window, invalidates all paint state, and repaints through the reference painter
without changing the run. The tile-paint prototype is disabled after a GPU failure.
A destroyed/unavailable operating-system window or a failure to create even an
SDL software renderer remains an unrecoverable display error, not a promised
recovery from arbitrary OS/device failures.

## Cost and bounds

Let P be drawable pixels, Q recorded quads, A newly cached mask pixels, M cached
mask pixels and U mask keys. Warm CPU command building is O(Q + text/layout and
existing per-cell cosmetic work), including expected hash lookup and key bytes.
Mask cache misses retain existing font/atlas rasterization cost and add O(A)
copy/conversion/upload. GPU work is O(covered pixel samples including overdraw),
plus driver costs; this is not O(changed cells) rendering. A frame still draws
its complete visible scene. Drawing commands and geometry buffers use O(Q)
memory and are reused. Geometry submits at most 16,383 quads per batch and
flushes on atlas-page transitions, preserving order.

CPU GPU-mask cache resets at the next frame boundary after 4096 keys or 32 MiB
of coverage. One frame may exceed this threshold before reset. GPU storage is
at most four 2048-square RGBA atlases (64 MiB), smaller on limited drivers;
exhaustion or an oversize glyph selects software painting. Atlas allocation and
upload are cold work, not included in a claimed warm constant. Cached masks
also exist in the reference font/mask caches. Existing framebuffer budget remains
33,554,432 pixels, with 4P bytes each for CPU framebuffer and fallback texture.
Resize invalidates cache keys and uploads new native-size masks; it is not free.

## Verification status

See the accompanying GPU test/report outputs for actual executed checks. The
isolated SDL test supports `SDL_VIDEODRIVER=offscreen` for OpenGL software-driver
validation and `SDL_VIDEODRIVER=dummy` for automatic no-accelerator fallback.
Physical Windows/Linux GPU performance and real driver/device loss require
separate platform testing. Do not present software-driver timing as hardware
GPU timing.

SDL device-reset events now request the same software reconstruction before any
next-loop drawable/texture query. If GPU or software drawing cannot create a
usable window renderer, the application still reports the display error.

The Linux offscreen driver was directly observed reporting a resized 1920×1080
window while retaining its original 1280×800 EGL pbuffer. Native image matrices
therefore use a fresh window for each resolution. Renderer cache invalidation
and painter resizing are unit-tested; real OS-window/HiDPI resize remains a
separate platform verification requirement.
