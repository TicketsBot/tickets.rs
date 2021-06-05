use serde::de::Unexpected;
use serde_json::Value;

pub fn to_unexpected<'a>(value: Value) -> Unexpected<'a> {
    match value {
        Value::Null => Unexpected::Other("null"),
        Value::Bool(b) => Unexpected::Bool(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Unexpected::Signed(i);
            }

            if let Some(u) = n.as_u64() {
                return Unexpected::Unsigned(u);
            }

            if let Some(f) = n.as_f64() {
                return Unexpected::Float(f);
            }

            panic!("Number is not i64 / u64 / f64 (impossible?)")
        }
        Value::String(_) => Unexpected::Other("string"), // TODO: FIX
        Value::Array(_) => Unexpected::Other("array"),   // TODO: FIX
        Value::Object(_) => Unexpected::Map,
        //_ => Unexpected::Other(std::any::type_name::<Value>()),
    }
}

// Does not allocate
pub fn empty_vec<T>() -> Vec<T> {
    Vec::with_capacity(0)
}
