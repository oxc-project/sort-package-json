use sort_package_json::sort_package_json;

#[test]
fn test_sort_package_json() {
  let input = include_str!("fixtures/package.json");
  let result = sort_package_json(input).expect("Failed to parse package.json");
  insta::assert_snapshot!(result);
}
