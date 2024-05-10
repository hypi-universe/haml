use std::fs;

pub fn resource_path(path: &str) -> String {
    format!("{}/tests/data/{}", env!("CARGO_MANIFEST_DIR"), path)
}

pub fn read_str_resource(path: &str) -> String {
    fs::read_to_string(resource_path(path)).expect(format!("Error reading test resource {}", path).as_str())
}
