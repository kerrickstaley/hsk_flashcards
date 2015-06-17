use cedict;

pub struct ChineseNote<'a> {
  pub ce: cedict::Entry<'a>,
  pub tags: Vec<String>,
}
