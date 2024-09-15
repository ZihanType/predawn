use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::Schema;

use crate::ToSchema;

macro_rules! wrapper_impl {
    ($($desc:tt)+) => {
        impl $($desc)+
        where
            T: ToSchema
        {
            fn title() -> Cow<'static, str> {
                T::title()
            }

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                T::schema(schemas, schemas_in_progress)
            }
        }
    };
}

wrapper_impl!(<'a, T: ?Sized> ToSchema for &'a T);
wrapper_impl!(<'a, T: ?Sized> ToSchema for &'a mut T);
wrapper_impl!(<T: ?Sized> ToSchema for Box<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::rc::Rc<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::rc::Weak<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::sync::Arc<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::sync::Weak<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::sync::Mutex<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::sync::RwLock<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::cell::Cell<T>);
wrapper_impl!(<T: ?Sized> ToSchema for std::cell::RefCell<T>);
wrapper_impl!(<'a, T: ?Sized + ToOwned> ToSchema for std::borrow::Cow<'a, T>);
wrapper_impl!(<T> ToSchema for std::num::Wrapping<T>);
wrapper_impl!(<T> ToSchema for std::cmp::Reverse<T>);
