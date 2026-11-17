# Animation video review fixtures

The opt-in core tests `export_new_animation_fixtures`, `export_amulet_animation_fixtures`, the authored cast fixtures, and enemy fixtures write before/after runs and actual emitted presentation receipts. They do not construct imitation animation events. Fixtures use controlled open rooms, high energy, and deliberately chosen actor states; they are not representative campaign balance or Windows-runtime tests.

The ignored desktop test `render_animation_review_videos` reads these exports and uses the actual observation, presentation-sequence and native Painter code to render 60-fps RGBA frames through FFmpeg. Ambient time is frozen at zero to isolate action animations. An identical base scene is cached until its staged view changes; each effects frame restores those exact base pixels before drawing. UI/state reveal follows supplied action snapshots; fixtures with no intermediate snapshots use the final snapshot at the final action boundary.

Videos can be cropped and enlarged from these actual pixels for readability, without adding invented effect shapes. External title strips and repeated playback are review aids, not game UI.

Complexity: for F output frames, C admitted effect cells and P output pixels, capture costs O(F*(P+C*G)) where current nova G is bounded by 12. New staged views incur normal observation/native base-paint costs. Memory is O(P+C+V) for pixels, receipt cells, and detached view snapshots; output is streamed to an encoder rather than storing all frames. Capture is test-only and opt-in.
