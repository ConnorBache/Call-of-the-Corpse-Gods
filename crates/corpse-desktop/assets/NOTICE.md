# Preserved reference assets

Except for the lossless WOFF2-to-sfnt container conversion noted below, these files were copied without modification from the frozen v48 game's assets directory. No publication or new licensing decision is made by this migration.

BrogueCE-ASCII.png is the reference's adapted printable ASCII artwork. Its existing CC BY-SA 4.0, attribution and upstream notices are preserved in brogue/. No Brogue renderer source is used.

Inconsolata-Variable.ttf is distributed with Inconsolata-OFL.txt. CorpseGlyphMono.ttf and CorpseGlyphSymbols.ttf retain NotoCore-OFL.txt; CorpseGlyphCJK.otf retains NotoCJK-OFL.txt. These are the already-renamed subsets from v48.

menu-logo.png and all 81 creature illustrations in creatures/ remain the project's authored artwork. Their inclusion does not establish mechanical support for those creatures.

IBMPlexMono-Regular.ttf is the reference IBM Plex Mono Regular WOFF2 decompressed to sfnt using fontTools, with the glyphs, names and outlines unchanged. IBMPlexMono-OFL.txt is retained. This matches the reference default UI family.

LiberationSerif-Regular.ttf and LiberationSans-Bold.ttf are the installed OFL Liberation fonts used as portable equivalents of the source manual's CSS serif and bold-sans fallback families. They are not claimed to be Georgia or Impact. Their existing copyright/OFL notice is preserved in Liberation-OFL.txt.

Native distribution derivatives: creatures-distribution/ contains quality90 WebP encodings at unchanged source dimensions; PNG masters in creatures/ remain unchanged. art-distribution.json records provenance. window-icon.png and corpse-gods.ico are platform icon representations of menu-logo.png, preserving its full composition and aspect ratio. See docs/native-embedded-art.md for encoding and regeneration details.

Bestiary typography: Inconsolata-Regular.ttf and Inconsolata-ExtraBold.ttf are static instances generated from Inconsolata-Variable.ttf at width100 and weights400/800 using fontTools. See Inconsolata-OFL.txt. They are not artificially stretched or emboldened. Existing creature art originals and proportions remain unchanged.

October 26 alpha revision: runtime creature art is lossless RGBA PNG under creatures-rgba/. creature-alpha-manifest.json separates reviewed segmentation from exact frozen legacy-reference extraction and untouched authored alpha. Original masters and historical WebP encodings remain unchanged. See docs/creature-alpha.md. No generative replacement artwork is used.
