use std::{fmt, ops::Deref};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NormalizedPath(String);

impl NormalizedPath {
    pub fn new(path: &str) -> Self {
        if path.is_empty() || path == "/" {
            return Self("/".to_string());
        }

        let segments = path.split('/');

        let mut path = String::new();

        for segment in segments {
            if segment.is_empty() {
                continue;
            }

            let segment = segment.trim();

            if segment.is_empty() {
                continue;
            }

            path.push('/');
            path.push_str(segment);
        }

        if path.is_empty() {
            path.push('/');
        }

        Self(path)
    }

    pub fn join(self, path: Self) -> Self {
        let prefix = self;
        let postfix = path;

        if prefix == "/" {
            postfix
        } else if postfix == "/" {
            prefix
        } else {
            let mut path = prefix.0;
            path.push_str(&postfix.0);

            Self(path)
        }
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

impl<'a> From<&'a str> for NormalizedPath {
    fn from(path: &'a str) -> Self {
        NormalizedPath::new(path)
    }
}

impl From<NormalizedPath> for String {
    fn from(path: NormalizedPath) -> Self {
        path.0
    }
}

impl Serialize for NormalizedPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NormalizedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <String as Deserialize<'de>>::deserialize(deserializer).map(|s| NormalizedPath::new(&s))
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
        fn join<'a>(prefix: &'a str, postfix: &'a str) -> NormalizedPath {
            NormalizedPath::new(prefix).join(NormalizedPath::new(postfix))
        }

        assert_eq!(join("", ""), "/");
        assert_eq!(join("/", ""), "/");
        assert_eq!(join("", "/"), "/");
        assert_eq!(join("/", "/"), "/");

        assert_eq!(join("/a", ""), "/a");
        assert_eq!(join("", "/a"), "/a");
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
