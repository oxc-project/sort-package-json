use std::env;
use std::fs;
use std::process;

fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() != 2 {
    eprintln!("Usage: {} <path-to-package.json>", args[0]);
    process::exit(1);
  }

  let file_path = &args[1];

  let contents = fs::read_to_string(file_path).unwrap_or_else(|err| {
    eprintln!("Error reading file '{}': {}", file_path, err);
    process::exit(1);
  });

  let sorted = sort_package_json::sort_package_json(&contents).unwrap_or_else(|err| {
    eprintln!("Error parsing JSON: {}", err);
    process::exit(1);
  });

  fs::write(file_path, sorted).unwrap_or_else(|err| {
    eprintln!("Error writing file '{}': {}", file_path, err);
    process::exit(1);
  });
}
