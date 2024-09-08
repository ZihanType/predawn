mod atomic;
mod bytes;
mod ffi;
mod json;
mod map;
mod non_zero;
mod option;
mod primitive;
mod seq;
mod set;
mod string;
mod time;
mod wrapper;

use macro_v::macro_v;

#[macro_v(pub(crate))]
macro_rules! forward_impl {
    ($left:ty => $right:ty) => {
        impl $crate::ToSchema for $left {
            fn schema(
                schemas: &mut ::std::collections::BTreeMap<String, ::openapiv3::Schema>,
                schemas_in_progress: &mut ::std::vec::Vec<::std::string::String>,
            ) -> ::openapiv3::Schema {
                <$right as $crate::ToSchema>::schema(schemas, schemas_in_progress)
            }
        }
    };
}
