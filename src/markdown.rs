use std::path::Path;

/// `std::fs::write`: truncates and replaces, nothing else.
pub struct MarkdownFile;

impl visper::MarkdownWriter for MarkdownFile {
    fn write(&mut self, file: &Path, body: &str) -> Result<(), String> {
        std::fs::write(file, body).map_err(|error| error.to_string())
    }
}
