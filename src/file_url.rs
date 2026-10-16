//! Recordings opened from Finder or dropped on the Dock icon reach Visper as
//! `file://` URLs, percent-encoded, Chinese names included.

use std::path::PathBuf;

/// The local path a `file://` URL names, or `None` for any other URL.
pub fn to_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    // `file:///Users/...` and `file://localhost/Users/...` are the same file.
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    Some(PathBuf::from(percent_decode(path)?))
}

fn percent_decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let hex = std::str::from_utf8(tail.get(..2)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_spaces_and_chinese_names() {
        assert_eq!(
            to_path("file:///Users/lin/%E5%BD%95%E9%9F%B3/%E5%91%A8%E4%BC%9A%20v2.m4a"),
            Some(PathBuf::from("/Users/lin/录音/周会 v2.m4a"))
        );
    }

    #[test]
    fn accepts_localhost_as_the_host() {
        assert_eq!(
            to_path("file://localhost/tmp/a.mp3"),
            Some(PathBuf::from("/tmp/a.mp3"))
        );
    }

    #[test]
    fn rejects_other_urls_and_broken_escapes() {
        assert_eq!(to_path("https://example.com/a.mp3"), None);
        assert_eq!(to_path("file://server/share/a.mp3"), None);
        assert_eq!(to_path("file:///tmp/a%2.mp3"), None);
        assert_eq!(to_path("file:///tmp/%FF.mp3"), None);
    }
}
