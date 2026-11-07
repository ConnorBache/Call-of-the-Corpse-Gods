//! No-output admission visitor: O(serialized nodes), O(depth) stack; no JSON tree,
//! byte buffer, float formatting or string cloning. Custom Serialize implementations
//! retain their own costs. Only canonical (not serde-skipped cache) fields are visited.
use serde::ser::*;
use serde::Serialize;
#[derive(Debug)] pub(super) struct NonFinite;
impl std::fmt::Display for NonFinite { /// O(1).
 fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("nonfinite or unsupported serialized anchor")}}
impl std::error::Error for NonFinite {}
impl Error for NonFinite { /// O(1), discard diagnostic formatting; fallback retains original errors.
 fn custom<T:std::fmt::Display>(_:T)->Self {Self}}
#[derive(Clone,Copy)] struct Check;
/// O(canonical nodes), O(depth) stack; no output allocation.
pub(super) fn all_finite<T:Serialize>(value:&T)->bool {value.serialize(Check).is_ok()}
impl Serializer for Check {
 type Ok=();type Error=NonFinite;
 type SerializeSeq=Self;type SerializeTuple=Self;type SerializeTupleStruct=Self;
 type SerializeTupleVariant=Self;type SerializeMap=Self;type SerializeStruct=Self;type SerializeStructVariant=Self;
 /// O(1), no output.
 fn serialize_bool(self,_:bool)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_i8(self,_:i8)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_i16(self,_:i16)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_i32(self,_:i32)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_i64(self,_:i64)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_i128(self,_:i128)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_u8(self,_:u8)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_u16(self,_:u16)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_u32(self,_:u32)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_u64(self,_:u64)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_u128(self,_:u128)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_char(self,_:char)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_str(self,_:&str)->Result<(),NonFinite>{Ok(())}
 /// O(1), no output.
 fn serialize_bytes(self,_:&[u8])->Result<(),NonFinite>{Ok(())}
 /// O(1), exact finite classification.
 fn serialize_f32(self,n:f32)->Result<(),NonFinite>{if n.is_finite(){Ok(())}else{Err(NonFinite)}}
 /// O(1), exact finite classification.
 fn serialize_f64(self,n:f64)->Result<(),NonFinite>{if n.is_finite(){Ok(())}else{Err(NonFinite)}}
 /// O(1), absent value.
 fn serialize_none(self)->Result<(),NonFinite>{Ok(())}
 /// O(value nodes), recurse without output.
 fn serialize_some<T:?Sized+Serialize>(self,v:&T)->Result<(),NonFinite>{v.serialize(self)}
 /// O(1).
 fn serialize_unit(self)->Result<(),NonFinite>{Ok(())}
 /// O(1).
 fn serialize_unit_struct(self,_:&'static str)->Result<(),NonFinite>{Ok(())}
 /// O(1).
 fn serialize_unit_variant(self,_:&'static str,_:u32,_:&'static str)->Result<(),NonFinite>{Ok(())}
 /// O(value nodes).
 fn serialize_newtype_struct<T:?Sized+Serialize>(self,_:&'static str,v:&T)->Result<(),NonFinite>{v.serialize(self)}
 /// O(value nodes).
 fn serialize_newtype_variant<T:?Sized+Serialize>(self,_:&'static str,_:u32,_:&'static str,v:&T)->Result<(),NonFinite>{v.serialize(self)}
 /// O(1).
 fn serialize_seq(self,_:Option<usize>)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_tuple(self,_:usize)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_tuple_struct(self,_:&'static str,_:usize)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_tuple_variant(self,_:&'static str,_:u32,_:&'static str,_:usize)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_map(self,_:Option<usize>)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_struct(self,_:&'static str,_:usize)->Result<Self,NonFinite>{Ok(self)}
 /// O(1).
 fn serialize_struct_variant(self,_:&'static str,_:u32,_:&'static str,_:usize)->Result<Self,NonFinite>{Ok(self)}
 /// O(1); its wire type is string, not a numeric JSON value.
 fn collect_str<T:?Sized+std::fmt::Display>(self,_:&T)->Result<(),NonFinite>{Ok(())}
}
impl SerializeSeq for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_element<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeTuple for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_element<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeTupleStruct for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_field<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeTupleVariant for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_field<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeStruct for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_field<T:?Sized+Serialize>(&mut self,_:&'static str,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeStructVariant for Check {type Ok=();type Error=NonFinite;
 /// O(value nodes), recursive no-output traversal.
 fn serialize_field<T:?Sized+Serialize>(&mut self,_:&'static str,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
impl SerializeMap for Check {type Ok=();type Error=NonFinite;
 /// O(key nodes).
 fn serialize_key<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(value nodes).
 fn serialize_value<T:?Sized+Serialize>(&mut self,v:&T)->Result<(),NonFinite>{v.serialize(*self)}
 /// O(1).
 fn end(self)->Result<(),NonFinite>{Ok(())}
}
