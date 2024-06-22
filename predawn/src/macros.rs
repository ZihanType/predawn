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

            InvalidContentTypeError($crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>),
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(
                        $name::$error(e) => ::core::fmt::Display::fmt(e, f),
                    )+

                    $name::InvalidContentTypeError(e) => ::core::fmt::Display::fmt(e, f),
                }
            }
        }

        impl ::std::error::Error for $name {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    $(
                        $name::$error(e) => Some(e),
                    )+

                    $name::InvalidContentTypeError(e) => Some(e),
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

        impl ::core::convert::From<$crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>> for $name {
            fn from(e: $crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>) -> Self {
                $name::InvalidContentTypeError(e)
            }
        }

        impl ::core::convert::From<::core::convert::Infallible> for $name {
            fn from(e: ::core::convert::Infallible) -> Self {
                match e {}
            }
        }

        impl $crate::response_error::ResponseError for $name {
            fn as_status(&self) -> $crate::__internal::http::StatusCode {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::as_status(e),
                    )+

                    $name::InvalidContentTypeError(e) => <
                        $crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>
                        as $crate::response_error::ResponseError
                    >::as_status(e),
                }
            }

            fn status_codes() -> ::std::collections::HashSet<$crate::__internal::http::StatusCode> {
                let mut status_codes = ::std::collections::HashSet::new();

                $(
                    status_codes.extend(<$error as $crate::response_error::ResponseError>::status_codes());
                )+

                status_codes.extend(
                    <
                        $crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>
                        as $crate::response_error::ResponseError
                    >::status_codes()
                );

                status_codes
            }

            #[doc(hidden)]
            fn inner(self, error_chain: &mut ::std::vec::Vec<&'static str>) -> $crate::error::BoxError {
                error_chain.push(::std::any::type_name::<Self>());

                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::inner(e, error_chain),
                    )+

                    $name::InvalidContentTypeError(e) => <
                        $crate::response_error::InvalidContentTypeError<{ $crate::count!($($error,)+) }>
                        as $crate::response_error::ResponseError
                    >::inner(e, error_chain),
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

        impl ::std::error::Error for $name {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
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
            fn as_status(&self) -> $crate::__internal::http::StatusCode {
                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::as_status(e),
                    )+
                }
            }

            fn status_codes() -> ::std::collections::HashSet<$crate::__internal::http::StatusCode> {
                let mut status_codes = ::std::collections::HashSet::new();

                $(
                    status_codes.extend(<$error as $crate::response_error::ResponseError>::status_codes());
                )+

                status_codes
            }

            #[doc(hidden)]
            fn inner(self, error_chain: &mut ::std::vec::Vec<&'static str>) -> $crate::error::BoxError {
                error_chain.push(::std::any::type_name::<Self>());

                match self {
                    $(
                        $name::$error(e) => <$error as $crate::response_error::ResponseError>::inner(e, error_chain),
                    )+
                }
            }
        }
    };
}
