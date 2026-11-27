//! HUD-only aliases. Never feed these names back into the simulation.
//! In particular “Ichor Leviathan” is both an old label and a new label, so
//! each original span is rewritten once; replacements are not scanned again.
use std::borrow::Cow;

const RENAMES: &[(&str,&str)] = &[
    ("Greater Ichor Leviathan","Ichor Leviathan"),
    ("Lesser Ichor Leviathan","Ichor Monster"),
    ("Ichor Leviathan","Ichor Kraken"),
    ("Short-legged Saidribee","Saidribee"),
    ("Short-Legged Saidribee","Saidribee"),
    ("Hemogoblin Bloodmancer","Hemomancer"),
    ("Teleportation Amulet","Vanish Amulet"),
    ("Amulet of Teleportation","Vanish Amulet"),
    ("Obstruction Amulet","Wall Amulet"),
    ("Amulet of Obstruction","Wall Amulet"),
];

/// O(L) for L input bytes with a fixed table. Borrow if no label changes.
pub fn present(text:&str)->Cow<'_,str> {
    if !RENAMES.iter().any(|(old,_)|text.contains(old)){return Cow::Borrowed(text);}
    let mut rest=text;
    let mut out=String::with_capacity(text.len());
    while !rest.is_empty() {
        if let Some((from,to))=RENAMES.iter().find(|(from,_)|rest.starts_with(from)) {
            out.push_str(to);
            rest=&rest[from.len()..];
        } else {
            let ch=rest.chars().next().unwrap();
            out.push(ch);
            rest=&rest[ch.len_utf8()..];
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hud_rename_leviathan_chain_only_rewrites_original_spans() {
        assert_eq!(present("Greater Ichor Leviathan meets Ichor Leviathan and Lesser Ichor Leviathan"),
            "Ichor Leviathan meets Ichor Kraken and Ichor Monster");
        assert_eq!(present("Blessed Greater Ichor Leviathan <heart> (marked)"),"Blessed Ichor Leviathan <heart> (marked)");
        assert_eq!(present("Mercuria’s Short-legged Saidribee"),"Mercuria’s Saidribee");
    }
}
