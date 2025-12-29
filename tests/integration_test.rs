use sort_package_json::{SortOptions, sort_package_json_with_options};
use std::fs;

fn sort(s: &str) -> String {
    sort_package_json_with_options(s, &SortOptions { pretty: true, sort_scripts: true })
        .expect("Failed to parse package.json")
}

#[test]
fn test_sort_package_json() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let result = sort(&input);
    insta::assert_snapshot!(result);
}

#[test]
fn test_idempotency() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let first_sort = sort(&input);
    let second_sort = sort(&first_sort);
    assert_eq!(first_sort, second_sort, "Sorting should be idempotent");
}
