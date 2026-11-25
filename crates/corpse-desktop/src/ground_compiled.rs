//! Per-observation pre-light ground descriptors. No cross-state color cache.
//! O(N * resolved lookup + owned glyph bytes), O(N) scratch. Neighbor smoothing
//! reuses each cell's ORIGINAL background, never the already smoothed/lit result.
use crate::appearance_profile::{Profile,MaterialParent as Parent,MaterialView};
use crate::render::Rgb;
use corpse_core::vision::ObservedCell;
#[derive(Debug)]
pub(super) struct Ground<'a>{pub glyph:String,pub fg:Rgb,pub bg:Rgb,pub fixture:bool,pub stone:bool,pub original:MaterialView<'a>,pub displayed:MaterialView<'a>}
fn raw_material<'a>(p:&'a Profile,o:&ObservedCell)->MaterialView<'a>{
    let tile=o.tile.as_deref().unwrap_or("floor");
    if tile == "speed-up" {
        let mut m=p.resolved_material("floor",Parent::None).unwrap_or_default();
        m.color=Some([95,211,211]);m.background=Some([12,49,51]);m.edge=Some([95,211,211]);
        m.edge_nonempty=true;m.glyph=Some(".");m.animation_speed=0.;
        m.color_default=true;m.background_default=true;m.edge_default=true;return m
    }
    if !matches!(tile,"up"|"down") && o.door_open.is_none()&&!o.grate&&!o.bridge&&(o.ice||o.puddles||o.hellfire){
        let mut m=p.resolved_material("floor",Parent::None).unwrap_or_default();
        let(f,b,g)=if o.hellfire{([227,56,36],[9,3,7],".")}else if o.ice{([168,220,233],[24,60,82],"_")}else{([117,155,165],[26,48,59],"~")};
        m.color=Some(f);m.background=Some(b);m.edge=Some(f);m.edge_nonempty=true;m.glyph=Some(g);m.animation_speed=0.;m.color_default=true;m.background_default=true;m.edge_default=true;return m
    }
    let ground_key;
    let(key,parent)=if let Some(open)=o.door_open{(if open{"fixture:door-open"}else{"fixture:door"},Parent::Door)}
        else if o.grate{("fixture:grate",Parent::None)}else if o.bridge{("fixture:bridge",Parent::None)}
        else if o.snow&&matches!(tile,"floor"|"grass"){("fixture:snow",Parent::None)}
        else if tile=="floor"{ground_key=o.substrate.as_ref().map(|s|format!("ground:{s}"));(ground_key.as_deref().unwrap_or("floor"),Parent::Floor)}
        else{(tile,Parent::None)};
    p.resolved_material(key,parent).or_else(||p.resolved_material(tile,Parent::None)).unwrap_or_default()
}
fn flesh_colors(tile:&str,substrate:Option<&str>)->Option<[Rgb;3]>{Some(match tile{
    "wall"=>[[180,163,154],[48,41,42],[208,189,176]],
    "floor"=>match substrate{Some("earth")=>[[187,124,114],[56,28,35],[187,124,114]],Some("sand")=>[[206,170,139],[67,37,42],[206,170,139]],Some("ash")=>[[180,161,170],[48,32,41],[180,161,170]],Some("soot")=>[[128,101,116],[30,17,27],[128,101,116]],_=>[[150,139,135],[35,28,32],[176,162,154]]},
    "water"=>[[65,125,224],[10,35,90],[110,165,247]],"shallow-water"=>[[151,214,245],[44,94,130],[183,232,255]],_=>return None
})}
pub(super) fn prepare<'a>(profile:&'a Profile,o:&ObservedCell,flesh:bool)->Ground<'a>{
    let original=raw_material(profile,o);let mut displayed=original;
    let tile=o.tile.as_deref().unwrap_or("floor");
    if flesh {
        let fixture=o.door_open.is_some()||o.grate||o.bridge||o.snow||o.ice||o.puddles||o.hellfire;
        if !fixture {if let Some([f,b,e])=flesh_colors(tile,o.substrate.as_deref()){
            if original.color_default{displayed.color=Some(f)}if original.background_default{displayed.background=Some(b)}if original.edge_default{displayed.edge=Some(e);displayed.edge_nonempty=true}
        }}
    }
    let(mut glyph,mut fg,mut bg,mut fixture,stone)=super::ground(o);
    if let Some(g)=displayed.glyph.filter(|g|!g.is_empty()){glyph=g.into();}
    if let Some(c)=displayed.color{fg=c}if let Some(c)=displayed.background{bg=c}
    if !fixture&&matches!(tile,"floor"|"grass"){
        let palette=match o.biome.as_deref(){Some("saidribee-nest")=>Some(([241,194,88],[66,43,19])),Some("flower-grove")=>Some(([149,203,114],[18,46,32])),Some("mountainous")=>Some(([162,177,193],[27,37,52])),Some("corn-dimension")=>Some(([226,197,246],[58,24,77])),_=>None};
        if let Some((f,b))=palette.filter(|_|!o.ice&&!o.snow&&!o.puddles&&!o.hellfire){if original.color_default{fg=f}if original.background_default{bg=b}}
        if o.flowers&&!o.snow&&!o.ice&&!o.puddles&&!o.hellfire{glyph="*".into();fg=match o.index%3{0=>[244,164,208],1=>[253,220,112],_=>[193,178,248]};}
    }
    if o.entrance{glyph="Ω".into();fg=[76,170,255];bg=[8,25,82];fixture=true;}
    Ground{glyph,fg,bg,fixture,stone,original,displayed}
}
