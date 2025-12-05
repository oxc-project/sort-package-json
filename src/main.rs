use std::env;
use std::fs;
use std::path::Path;
use std::process;

use ignore::WalkBuilder;

#[allow(clippy::print_stderr)]
fn main() {
    let search_path = env::current_dir().unwrap_or_else(|err| {
        eprintln!("Error getting current directory: {}", err);
        process::exit(1);
    });

    // Find all package.json files
    let mut found_files = 0;
    let mut sorted_files = 0;
    let mut errors = 0;

    for entry in WalkBuilder::new(search_path)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_name() == "package.json")
    {
        found_files += 1;
        let file_path = entry.path();

        match process_file(file_path) {
            Ok(()) => {
                sorted_files += 1;
                eprintln!("✓ Sorted: {}", file_path.display());
            }
            Err(err) => {
                errors += 1;
                eprintln!("✗ Error processing {}: {}", file_path.display(), err);
            }
        }
    }

    eprintln!("\nSummary:");
    eprintln!("  Found: {}", found_files);
    eprintln!("  Sorted: {}", sorted_files);
    eprintln!("  Errors: {}", errors);

    if errors > 0 {
        process::exit(1);
    }
}

fn process_file(file_path: &Path) -> Result<(), String> {
    let contents =
        fs::read_to_string(file_path).map_err(|err| format!("Failed to read: {}", err))?;

    let sorted = sort_package_json::sort_package_json(&contents)
        .map_err(|err| format!("Failed to parse JSON: {}", err))?;

    fs::write(file_path, sorted).map_err(|err| format!("Failed to write: {}", err))?;

    Ok(())
}
