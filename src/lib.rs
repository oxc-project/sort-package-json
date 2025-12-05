pub fn sort_package_json(input: &str) -> Result<String, serde_json::Error> {
  let value: serde_json::Value = serde_json::from_str(input)?;
  serde_json::to_string_pretty(&value)
}
