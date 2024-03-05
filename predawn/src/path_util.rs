use std::collections::HashSet;

use crate::normalized_path::NormalizedPath;

fn inner_convert_path<'a>(
    controller_path: &'a str,
    method_path: &'a str,
) -> Result<(NormalizedPath, NormalizedPath), String> {
    let controller_path = NormalizedPath::new(controller_path);
    let method_path = NormalizedPath::new(method_path);

    if controller_path == "/" && method_path == "/" {
        return Ok((NormalizedPath::new("/"), NormalizedPath::new("/")));
    }

    let mut oai_path = String::new();
    let mut route_path = String::new();
    let mut path_vars = HashSet::new();

    // skip one because paths always start with `/` so `/a/b` would become ["", "a", "b"]
    for segment in controller_path
        .split('/')
        .skip(1)
        .chain(method_path.split('/').skip(1))
    {
        if segment.is_empty() {
            continue;
        }

        if let Some(var) = segment.strip_prefix(':') {
            oai_path.push_str("/{");
            oai_path.push_str(var);
            oai_path.push('}');

            route_path.push_str("/:");
            route_path.push_str(var);

            if !path_vars.insert(var) {
                return Err(var.to_string());
            }
        } else {
            oai_path.push('/');
            oai_path.push_str(segment);

            route_path.push('/');
            route_path.push_str(segment);
        }
    }

    Ok((
        NormalizedPath::only_used_in_convert_path(oai_path),
        NormalizedPath::only_used_in_convert_path(route_path),
    ))
}

#[doc(hidden)]
#[track_caller]
pub fn convert_path<'a>(
    controller_path: &'a str,
    method_path: &'a str,
    controller_name: &'static str,
    method_name: &'static str,
) -> (NormalizedPath, NormalizedPath) {
    match inner_convert_path(controller_path, method_path) {
        Ok(o) => o,
        Err(var) => {
            panic!(
                "controller: `{}`, method: `{}`, controller_path: `{}`, method_path: `{}` has duplicate path variable `{}`",
                controller_name, method_name, controller_path, method_path, var
            );
        }
    }
}

#[track_caller]
pub(crate) fn validate_path(path: &str) {
    if path.contains(':') {
        panic!("invalid path: `{}`, path cannot contain colons (:)", path);
    }

    if path.contains('*') {
        panic!(
            "invalid path: `{}`, path cannot contain wildcards (*)",
            path
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inner_convert_path() {
        assert_eq!(inner_convert_path("", "").unwrap().1, "/");
        assert_eq!(inner_convert_path("/", "/").unwrap().1, "/");
        assert_eq!(inner_convert_path("/abc", "").unwrap().1, "/abc");

        let (oai_path, route_path) = inner_convert_path("/:abc", "").unwrap();
        assert_eq!(oai_path, "/{abc}");
        assert_eq!(route_path, "/:abc");

        assert_eq!(inner_convert_path("", "/abc").unwrap().1, "/abc");
        assert_eq!(inner_convert_path("/abc", "/def").unwrap().1, "/abc/def");
        assert_eq!(inner_convert_path("/", "/ghi").unwrap().1, "/ghi");
        assert_eq!(inner_convert_path("/abc/", "/").unwrap().1, "/abc");
        assert_eq!(inner_convert_path("/abc/", "/def").unwrap().1, "/abc/def");
        assert_eq!(inner_convert_path("/abc/", "/def/").unwrap().1, "/abc/def");

        let (oai_path, route_path) = inner_convert_path("/:abc/", "/:def/").unwrap();
        assert_eq!(oai_path, "/{abc}/{def}");
        assert_eq!(route_path, "/:abc/:def");
    }
}
