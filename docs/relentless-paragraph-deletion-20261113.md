# Relentless Pursuit paragraph deletion, 2026-11-13

Asked whether to remove the whole quoted implementation paragraph or only its label, the owner answered "Remove that whole thing". This supersedes the earlier hold for this paragraph.

Removed exactly the paragraph beginning "Implementation: The path follows the moving target" and its preceding blank separator from pelian-relentless-pursuit.description in abilities-v50.json. First paragraph, effectSummary, flags, cost, all other catalog fields and mechanics remain unchanged. The exact paragraph occurred once in this catalog and not in the checked ability_ui.rs, creature_abilities_ui.rs or progression.rs paths. progression.rs loads this catalog for revision >=50.

Verified JSON parsing and exactly one changed string field; original Git blob and minimal deletion compared. No simulation or Rust code changed. Native/runtime tests NOT RUN. This is the older R51 review branch, not the missing latest integrated R52 source; no whole R52 file substituted and no main merge.
