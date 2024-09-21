#[doc(hidden)]
#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:ident, $($xs:ident,)* ) => (1usize + $crate::count!($($xs,)*));
}

#[macro_export]
macro_rules! define_from_request_error {
    (
        name: $name:ident,
        errors: [$($error:ident),+ $(,)?]$(,)?
    ) => {
        #[derive(Debug)]
        pub enum $name {
            $(
                $error($error),
            )+

            InvalidContentType($crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>),
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(
                        $name::$error(e) => ::core::fmt::Display::fmt(e, f),
                    )+

                    $name::InvalidContentType(e) => ::core::fmt::Display::fmt(e, f),
                }
            }
        }

        impl ::core::error::Error for $name {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                match self {
                    $(
                        $name::$error(e) => Some(e),
                    )+

                    $name::InvalidContentType(e) => Some(e),
                }
            }
        }

        $(
            impl ::core::convert::From<$error> for $name {
                fn from(e: $error) -> Self {
                    $name::$error(e)
                }
            }
        )+

        impl ::core::convert::From<$crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>> for $name {
            fn from(e: $crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>) -> Self {
                $name::InvalidContentType(e)
            }
        }

        impl ::core::convert::From<::core::convert::Infallible> for $name {
            fn from(e: ::core::convert::Infallible) -> Self {
                match e {}
            }
        }

        impl $crate::response_error::ResponseError for $name {
            fn as_status(&self) -> $crate::http::StatusCode {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::as_status(e),
                    )+

                    $name::InvalidContentType(e) => <
                        $crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>
                        as $crate::response_error::ResponseError
                    >::as_status(e),
                }
            }

            fn status_codes(codes: &mut ::std::collections::BTreeSet<$crate::http::StatusCode>) {
                $(
                    <$error as $crate::response_error::ResponseError>::status_codes(codes);
                )+

                <
                    $crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>
                    as
                    $crate::response_error::ResponseError
                >::status_codes(codes);
            }

            #[doc(hidden)]
            fn inner(self) -> $crate::error::BoxError {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::inner(e),
                    )+

                    $name::InvalidContentType(e) => <
                        $crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>
                        as
                        $crate::response_error::ResponseError
                    >::inner(e),
                }
            }

            fn error_stack(&self, stack: &mut $crate::error_stack::ErrorStack) {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::error_stack(e, stack),
                    )+

                    $name::InvalidContentType(e) => <
                        $crate::response_error::InvalidContentType<{ $crate::count!($($error,)+) }>
                        as
                        $crate::response_error::ResponseError
                    >::error_stack(e, stack),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_into_response_error {
    (
        name: $name:ident,
        errors: [$($error:ident),+ $(,)?]$(,)?
    ) => {
        #[derive(Debug)]
        pub enum $name {
            $(
                $error($error),
            )+
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(
                        $name::$error(e) => ::core::fmt::Display::fmt(e, f),
                    )+
                }
            }
        }

        impl ::core::error::Error for $name {
            fn source(&self) -> Option<&(dyn ::core::error::Error + 'static)> {
                match self {
                    $(
                        $name::$error(e) => Some(e),
                    )+
                }
            }
        }

        $(
            impl ::core::convert::From<$error> for $name {
                fn from(e: $error) -> Self {
                    $name::$error(e)
                }
            }
        )+

        impl ::core::convert::From<::core::convert::Infallible> for $name {
            fn from(e: ::core::convert::Infallible) -> Self {
                match e {}
            }
        }

        impl $crate::response_error::ResponseError for $name {
            fn as_status(&self) -> $crate::http::StatusCode {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::as_status(e),
                    )+
                }
            }

            fn status_codes(codes: &mut ::std::collections::BTreeSet<$crate::http::StatusCode>) {
                $(
                    <$error as $crate::response_error::ResponseError>::status_codes(codes);
                )+
            }

            #[doc(hidden)]
            fn inner(self) -> $crate::error::BoxError {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::inner(e),
                    )+
                }
            }

            fn error_stack(&self, stack: &mut $crate::error_stack::ErrorStack) {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::error_stack(e, stack),
                    )+
                }
            }
        }
    };
}
