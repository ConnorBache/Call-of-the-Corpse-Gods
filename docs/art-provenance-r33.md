# Revision 33 — new creature art provenance

These three assets were created with built-in image generation, with a transparent
background requested. Night Crawler and Saidribee Queen use an initial generation
followed by the selected targeted framing/alpha edit; Skin Walker uses the accepted
generation with the user-approved localized pelvic blur. The accepted PNG pixels
are preserved unchanged in both the master and runtime copies. Existing creature
art was inspected for visual continuity, not repainted or reused as a finished new
illustration. No post-generation alpha thresholding, dark-pixel keying or procedural
stand-in was applied.

## Final project assets

All three files are 1254 × 1254 RGBA PNGs. Paths below are relative to the project
root. Each listed SHA-256 applies to both that asset's master and runtime file.

### Night Crawler

- Master: `crates/corpse-desktop/assets/creatures/night-crawler.png`
- Runtime: `crates/corpse-desktop/assets/creatures-rgba/night-crawler.png`
- SHA-256: `821a18e42f729397d82c480c72deb2b37bd9050a36f8c3696dca2211a9e36da6`

### Skin Walker

- Master: `crates/corpse-desktop/assets/creatures/skin-walker.png`
- Runtime: `crates/corpse-desktop/assets/creatures-rgba/skin-walker.png`
- SHA-256: `e34e3e78ace398ff1bf6f90a547a8f4475a3f457fef8840c06ab9d7d4d73f856`

### Saidribee Queen

- Master: `crates/corpse-desktop/assets/creatures/saidribee-queen.png`
- Runtime: `crates/corpse-desktop/assets/creatures-rgba/saidribee-queen.png`
- SHA-256: `37bc5766c5eb42feb85aaf5689f0273c31c836eb1ba4fd9355a8b62bd5e1f578`

## Accepted treatment and alpha limits

Skin Walker's small pelvic/genital area is deliberately obscured by the
user-approved localized blur; the rest retains its sharp pixel texture. No
explicit genital details are shown. Night Crawler was not changed by that choice.

The assets have genuine zero-alpha background regions, but are **not binary-alpha
cutouts**. Many solid-looking pixels remain near-opaque (typically alpha 252–253,
rather than exactly 255), and faint low-alpha fringe/speckles remain. Four-background
inspection used red, white, gray and black; clean normal-scale appearance does not
prove perfect opacity. Skin Walker's top/bottom framing is narrower than requested.
These limitations are retained honestly rather than silently repaired or described
as having satisfied every prompt constraint.

## Prompts for the selected assets

Prompts are recorded verbatim below as creative/reproduction instructions, not as
claims that every requested alpha or margin property was achieved. Only the prompt
chain for each selected asset is retained here.

### Night Crawler — initial generation

```text
Use case: stylized-concept
Asset type: production dark-fantasy dungeon bestiary creature illustration, one isolated creature.
Primary request: NIGHT CRAWLER — a terrible nonhumanoid creature of tentacles and sharp appendages. Its tangled muscular tentacles terminate in or bear many dagger-like natural sharp blades; these are hollow piercing feeding limbs. Include one clearly thinner, unbladed reproductive tentacle as a minor natural anatomical part among the other tentacles, not aroused, not a sexual act or focal detail. No victims.
Subject: one whole nonhumanoid tangled tentacled horror; no human torso, human legs, clothing, armor, or handheld weapons. A coherent central body joins the radiating and curling tentacles, many wicked tapering dagger-sharp appendages. Dark organic purple-brown flesh with desaturated bone-colored sharp ends and precise pale texture highlights, visible dark inner anatomy.
Style/medium: extremely detailed grim dark fantasy pixel-art bestiary rendering: crisp visible square pixel clusters, controlled dither shading, strong anatomy and volume, muted bone/ash/gray-mauve/umber palette. Match the established dense finely pixelated retro creature sprite look, NOT smooth painted artwork, not cartoon, not vector.
Composition/framing: centered full creature on square canvas, three-quarter specimen view, all tentacle tips and dagger ends visible with generous transparent safety margins on every side; strong readable silhouette suitable for a game creature page.
Scene/backdrop: genuine transparent RGBA background, alpha zero outside the creature. No scenery, ground, cast shadow, frame, UI, captions, letters, watermark, baked black matte, checkerboard pixels, blur, edge glow or halo. Preserve dark flesh as opaque creature pixels; do not confuse dark anatomy with background.
Lighting/mood: menacing anatomical body-horror specimen, non-erotic, no sexual pose or genital closeup, no depicted violence.
```

### Night Crawler — selected cleanup edit

```text
Edit target: the existing Night Crawler illustration. Precise production cleanup only. Preserve this exact creature design, anatomy, texture, muted purple-brown colors, dagger-like bony appendages and one minor thinner unbladed reproductive tentacle; do not redesign or add content.
Make the complete creature about 82 percent of the square canvas width and height, centered, leaving clear transparent safety padding on every side. No tips may touch the image edges. Keep fine, sharp square pixel-art texture; no resampling blur.
Fix the alpha channel to a clean sprite cutout: fully opaque pixels throughout all of the creature including its darkest flesh and inner tentacles, fully alpha-zero empty background and gaps. At most a single pixel of edge antialiasing. Do not remove dark anatomy. No low-alpha speckles anywhere in empty space. No baked black/white/color matte, no checkerboard, no halo or fringe. This must composite cleanly onto solid white, black, gray and bright red.
Transparent RGBA background; no scenery, floor, shadow, text or frame.
```

Visual references inspected: Pale Biter, Pale Spitter and Killspawn master art.
The cleanup used the preceding Night Crawler generation as its image reference.

### Skin Walker — accepted generation

```text
Use case: stylized-concept
Asset type: one production dark-fantasy bestiary creature sprite, single full-body figure on transparent RGBA.
Primary request: SKIN WALKER, a distorted and uncanny ADULT male human-shaped horror. Clearly mature adult facial features and body proportions, pale sallow skin, warped long limbs, two arms and two legs. Eyes are darkened hollow black. The mouth is a circular lamprey opening with small concentric pointed teeth. A long tongue hangs out of that mouth down over the chest to the midsection. Neutral menacing standing posture, arms loose and away from torso, slight three-quarter front view. No victim, violence, sexual activity, arousal, or erotic presentation.
Modesty treatment explicitly requested by the user: the figure is otherwise unclothed, but the entire small pelvic/genital region is deliberately obscured by a localized opaque soft blur, so adult anatomy is only implied and NO genital details are visible. This small blurred patch blends with the figure's flesh colors. Do not add clothes, underwear, a loincloth, a censor label, or a black bar. The rest of the figure stays sharply rendered. Do not blur the whole image and do not emphasize the obscured region.
Style: dense highly detailed grim retro dark fantasy pixel art, clearly visible crisp square pixel clusters, disciplined dither shading, anatomical volume with muted bone-white, ash, gray-mauve and umber. Closely match a detailed pixel-art game bestiary rather than smooth painting, illustration with vector outlines, cartoon or 3D render.
Composition: one entire creature centered on a square canvas, head, hands and feet all safely within frame; creature occupies about 82 percent of canvas height with clean empty safety margins all around.
Background and alpha: actual transparent RGBA background with alpha zero outside the creature and fully opaque creature body, including all dark anatomy and the localized pelvis blur. Clean isolated cutout with no baked background, colored matte, checkerboard pattern, fringe, glow, halo, shadow, ground or scenery.
No text, captions, watermark, UI, frame, other characters, exposed organs or accessories.
```

### Saidribee Queen — initial generation

```text
Use case: stylized-concept.
Asset type: game bestiary portrait sprite for The Call of the Corpse Gods, matching the existing Saidribee insect family.
Primary request: one formidable Saidribee Queen, an unmistakably adult nonhumanoid bee/wasp monster, alone. She is a biological insect queen rather than a person or a costumed royal: very large heavy elongated segmented abdomen with broad golden-ochre and charcoal bands, thick golden shaggy thorax, dark shiny compound eyes, expressive predatory mandibles, two long jointed antennae, six insect limbs with hooked claws, and two pairs of veined amber-gray membranous wings. Broad-bodied and imposing, with a sweeping pointed stinger. Visual family traits: warm honey-yellow fur, dark brown chitinous armor, bronze-brown claws and mandibles, pale warm highlights, dark brown lowlights.
Style/medium: detailed sharp pixel-art dark-fantasy bestiary sprite, a cohesive richly shaded illustration made from visibly crisp pixel clusters and stepped pixel edges; no blur, no painterly smears, no smooth 3D rendering.
Composition/framing: full body in a readable three-quarter view facing slightly left, abdomen sweeping back to the right. Wings spread enough to read their veins. Centered single creature in a square canvas, all wing tips, antennae, legs, claws, and stinger safely inside the frame with generous clear margin. The enlarged queen abdomen must make the silhouette distinct from a worker bee.
Scene/backdrop: genuinely empty transparent RGBA background with alpha zero outside the creature, including all gaps between legs and wings; no colored background, no black matte, no checkerboard graphic.
Lighting/mood: dimensional warm bestiary illumination with preserved dark anatomical details and clear silhouette, threatening but biologically coherent.
Constraints: insect anatomy only; no human breasts, human torso, human face, human hands, or genitalia; no clothing, jewelry, literal crown, scepter, armor equipment, throne, honey jar, honey drops, child creatures, workers, eggs, extra creatures, or props. No floor, scenery, ground contact shadow, cast shadow, glows, particles, text, watermark, or decorative border. Do not flatten any dark insect details into transparency.
```

### Saidribee Queen — selected framing/alpha edit

```text
Use case: precise-object-edit.
Edit the attached Saidribee Queen sprite. Preserve this exact creature identity, species anatomy, pose, color palette, enlarged striped abdomen, all six hooked insect legs, antennae, mandibles, compound eyes, four veined wings and stinger, and detailed pixel-art dark-fantasy style. Do not redesign or add anything.
Make only the production-cutout corrections: shrink the whole intact creature within the square canvas so its full bounding silhouette occupies at most 78% of the width and height, centered, with a generous clearly empty transparent margin on EVERY side. Keep all appendages fully visible. Deliver genuine transparent RGBA: alpha exactly zero everywhere outside the creature and inside the spaces between appendages, without any low-alpha haze around the empty canvas. Make the insect's solid body, fur, chitin, eyes, mandibles and claws fully opaque; only natural wing translucency and a narrow clean edge antialias may be partial alpha. Preserve all dark anatomical detail; do not treat dark pixels as background. No backdrop, floor, shadow, checkerboard graphic, glow, particles, props, extra creatures, clothing, crown, jewelry, human anatomy, text, watermark or border. Keep crisp stepped pixel-art edges, not softened painted or airbrushed edges.
```

Visual references inspected: Saidribee Guardian, Long-legged Saidribee and
Short-legged Saidribee master art. Family traits informed the initial prompt; the
selected edit used the preceding Queen generation as its image reference.
