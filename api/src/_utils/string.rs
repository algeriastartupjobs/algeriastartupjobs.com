pub fn slugify(s: &String) -> String {
  let filtered = s
    .chars()
    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
    .collect::<String>();
  let mut slug = slug::slugify(filtered)
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
    .collect::<String>();
  slug.truncate(50);
  slug
}

pub fn escape_double_quote(s: &String) -> String {
  s.replace('\"', "\\\"")
}

pub fn escape_new_line(s: &String) -> String {
  s.replace('\n', "\\n")
}

pub fn escape_new_line_with_br(s: &String) -> String {
  s.replace('\n', "<br>")
}

// @TODO-ZM: change this to get_searchable_words
pub fn get_words(paragraph: &str) -> impl Iterator<Item = &'_ str> {
  paragraph
    .split(|c: char| !c.is_alphanumeric())
    .filter(|s| !s.is_empty())
}

// @TODO-ZM: filter out common words
pub fn get_searchable_words(paragraph: &String) -> Vec<String> {
  paragraph
    .split(|c: char| !c.is_alphanumeric())
    .map(|s| s.to_string())
    .filter(|s| !s.is_empty())
    .collect()
}
