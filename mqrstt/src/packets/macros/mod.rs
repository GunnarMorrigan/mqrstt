mod properties_macros;
mod reason_code_macros;


pub(crate) use reason_code_macros::*;
pub(crate) use properties_macros::*;

// macro_rules! assert_length {
//     ($len:ident, $read:expr) => {
//         if len != (read) {
//             return Err(DeserializeError::InvalidLength(std::any::type_name::<Self>(), len, read));
//         }
//     };
// }