use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct NormalizedPath(String);

impl NormalizedPath {
    pub(crate) fn only_used_in_convert_path(path: String) -> Self {
        NormalizedPath(path)
    }

    pub fn new(path: &str) -> Self {
        if path.is_empty() || path == "/" {
            return Self("/".to_string());
        }

        let mut new_path = String::new();

        for segment in path.split('/') {
            if segment.is_empty() {
                continue;
            }

            let segment = segment.trim();

            if segment.is_empty() {
                continue;
            }

            new_path.push('/');
            new_path.push_str(segment);
        }

        if new_path.is_empty() {
            new_path.push('/');
        }

        Self(new_path)
    }

    pub fn join(self, path: Self) -> Self {
        let Self(prefix) = self;
        let Self(path) = path;

        Self(
            if prefix == "/" {
                path
            } else if path == "/" {
                prefix
            } else {
                format!("{}{}", prefix, path)
            },
        )
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for NormalizedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Deref for NormalizedPath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NormalizedPath> for String {
    fn from(path: NormalizedPath) -> Self {
        path.0
    }
}

impl PartialEq<&str> for NormalizedPath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<NormalizedPath> for &str {
    fn eq(&self, other: &NormalizedPath) -> bool {
        *self == other.0
    }
}

impl PartialEq<String> for NormalizedPath {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<NormalizedPath> for String {
    fn eq(&self, other: &NormalizedPath) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(NormalizedPath::new(""), "/");
        assert_eq!(NormalizedPath::new("/"), "/");
        assert_eq!(NormalizedPath::new("//"), "/");
        assert_eq!(NormalizedPath::new(" / / "), "/");

        assert_eq!(NormalizedPath::new("a"), "/a");
        assert_eq!(NormalizedPath::new("/a"), "/a");
        assert_eq!(NormalizedPath::new("a/"), "/a");
        assert_eq!(NormalizedPath::new("/a/"), "/a");
        assert_eq!(NormalizedPath::new("//a//"), "/a");
        assert_eq!(NormalizedPath::new(" // a/ /"), "/a");

        assert_eq!(NormalizedPath::new("a/b"), "/a/b");
        assert_eq!(NormalizedPath::new("/a/b"), "/a/b");
        assert_eq!(NormalizedPath::new("a/b/"), "/a/b");
        assert_eq!(NormalizedPath::new("/a/b/"), "/a/b");
        assert_eq!(NormalizedPath::new("//a//b//"), "/a/b");
        assert_eq!(NormalizedPath::new(" / /a // b/ / "), "/a/b");

        assert_eq!(
            NormalizedPath::new(" / /a // hello world /  d / "),
            "/a/hello world/d"
        );
    }

    #[test]
    fn test_join_path() {
        fn join(prefix: &str, path: &str) -> NormalizedPath {
            NormalizedPath::new(prefix).join(NormalizedPath::new(path))
        }

        assert_eq!(join("", ""), "/");
        assert_eq!(join("/", ""), "/");
        assert_eq!(join("", "/"), "/");
        assert_eq!(join("/", "/"), "/");

        assert_eq!(join("/a", "/"), "/a");
        assert_eq!(join("/", "/a"), "/a");
        assert_eq!(join("/a/", "/"), "/a");
        assert_eq!(join("/", "/a/"), "/a");

        assert_eq!(join("/a", "/b"), "/a/b");
        assert_eq!(join("/a/", "/b"), "/a/b");
        assert_eq!(join("/a", "/b/"), "/a/b");
        assert_eq!(join("/a/", "/b/"), "/a/b");
    }
}
