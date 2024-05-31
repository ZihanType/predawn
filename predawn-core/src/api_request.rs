use std::collections::BTreeMap;

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};

use crate::{
    body::RequestBody,
    media_type::MultiRequestMediaType,
    openapi::{self, Parameter, Schema},
    private::{ViaRequest, ViaRequestHead},
    request::{BodyLimit, Head, LocalAddr, OriginalUri, RemoteAddr},
};

pub trait ApiRequestHead {
    fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>>;
}

pub trait ApiRequest<M = ViaRequest> {
    fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>>;

    fn request_body(schemas: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody>;
}

impl<T> ApiRequest<ViaRequestHead> for T
where
    T: ApiRequestHead,
{
    fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
        T::parameters(schemas)
    }

    fn request_body(_: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
        None
    }
}

impl ApiRequest for RequestBody {
    fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(_: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
        None
    }
}

macro_rules! some_request {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ApiRequest for $ty {
                fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
                    None
                }

                fn request_body(schemas: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
                    Some(openapi::RequestBody {
                        content: <$ty as MultiRequestMediaType>::content(schemas),
                        required: true,
                        ..Default::default()
                    })
                }
            }
        )+
    };
}

some_request![Bytes, Vec<u8>, String];

macro_rules! none_request_head_for_ref_and_owned {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<'a> ApiRequestHead for &'a $ty {
                fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
                    None
                }
            }

            impl ApiRequestHead for $ty {
                fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
                    None
                }
            }
        )+
    };
}

none_request_head_for_ref_and_owned![Head, Uri, Method, HeaderMap, OriginalUri];

macro_rules! none_request_head_for_owned {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ApiRequestHead for $ty {
                fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
                    None
                }
            }
        )+
    };
}

none_request_head_for_owned![Version, LocalAddr, RemoteAddr, BodyLimit];

macro_rules! optional_parameters {
    ($ty:ty) => {
        fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
            let mut parameters = <$ty>::parameters(schemas)?;

            parameters.iter_mut().for_each(|parameter| match parameter {
                Parameter::Query { parameter_data, .. } => parameter_data.required = false,
                Parameter::Header { parameter_data, .. } => parameter_data.required = false,
                Parameter::Path { parameter_data, .. } => parameter_data.required = false,
                Parameter::Cookie { parameter_data, .. } => parameter_data.required = false,
            });

            Some(parameters)
        }
    };
}

impl<T: ApiRequestHead> ApiRequestHead for Option<T> {
    optional_parameters!(T);
}

impl<T: ApiRequest> ApiRequest for Option<T> {
    optional_parameters!(T);

    fn request_body(schemas: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
        let mut request_body = T::request_body(schemas)?;
        request_body.required = false;
        Some(request_body)
    }
}

impl<T, E> ApiRequestHead for Result<T, E>
where
    T: ApiRequestHead,
{
    fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
        T::parameters(schemas)
    }
}

impl<T, E> ApiRequest for Result<T, E>
where
    T: ApiRequest,
{
    fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Option<Vec<Parameter>> {
        T::parameters(schemas)
    }

    fn request_body(schemas: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
        T::request_body(schemas)
    }
}
