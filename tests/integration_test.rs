use sort_package_json::sort_package_json;
use std::fs;

#[test]
fn test_sort_package_json() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let result = sort_package_json(&input).expect("Failed to parse package.json");
    insta::assert_snapshot!(result);
}

#[test]
fn test_idempotency() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let first_sort = sort_package_json(&input).expect("Failed to parse package.json on first sort");
    let second_sort =
        sort_package_json(&first_sort).expect("Failed to parse package.json on second sort");
    assert_eq!(first_sort, second_sort, "Sorting should be idempotent");
}
