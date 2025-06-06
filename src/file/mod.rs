use std::fs;

pub struct File;

impl File {
  pub fn read_file(file: &str) -> String {
    fs::read_to_string(file).expect("Unable to read file")
  }
}
