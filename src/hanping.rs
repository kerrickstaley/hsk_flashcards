use cedict;
use chinese_note;
use std;

pub fn get_chinese_notes<'a>(wordlist: &'a str, extra_entries: &'a str)
    -> Vec<chinese_note::ChineseNote<'a>> {
  let dict = cedict::Dict::new_with_extra_entries(extra_entries);
  let mut rv = Vec::new();
  for line in wordlist.split("\n") {
    let pl = match parse_line(&line) {
      Ok(pl) => pl,
      Err(s) => {
        panic!("{}", s);
      }
    };
    let entries = dict.search(
        cedict::DictSearchParams{
            trad: Some(&pl.trad), simp: Some(&pl.simp), pinyin: Some(&pl.pinyin)});
    if entries.len() != 1 {
      println!("Warning: number of entries for {:?} was {}; not exactly 1.", pl, entries.len());
    }
    if entries.len() > 0 {
      // entries[entries.len() - 1] causes it to prefer entries with lowercase pinyin, e.g.
      //   乾 干 [gan1] /dry/clean/in vain/dried food/foster/adoptive/to ignore/
      // will be preferred over
      //   乾 干 [Gan1] /surname Gan/
      rv.push(chinese_note::ChineseNote{ce: entries[entries.len() - 1].clone(), tags: vec!()});
    }
  }
  rv // TODO
}

pub struct ParsedLine {
  pub trad: String,
  pub simp: String,
  pub pinyin: String,
}

impl std::fmt::Debug for ParsedLine {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    write!(
        formatter,
        "ParsedLine {{ trad: \"{}\", simp: \"{}\", pinyin: \"{}\" }}",
        self.trad, self.simp, self.pinyin)
  }
}

// TODO: can we make this only pub for testing?
pub fn parse_line(line: &str) -> Result<ParsedLine, String> {
  let line_re = regex!(r"^(.+?)(?: \[(.+?)\])? +(.+)$");
  let caps = match line_re.captures(line) {
    Some(caps) => caps,
    None => { return Err("Could not parse Hanping word list line: ".to_string() + line) },
  };
  let mut rv = ParsedLine{
    trad: String::new(),
    simp: String::new(),
    pinyin: String::new(),
  };
  rv.trad = caps.at(1).unwrap().to_string();
  rv.simp = match caps.at(2) {
    Some(simp_dash) =>
        simp_dash.chars().zip(rv.trad.chars()).map(
            |item| if item.0 != '-' { item.0 } else { item.1 }).collect(),
    None => rv.trad.to_string(),
  };
  let rest = caps.at(3).unwrap();
  let mut syllables = rv.trad.chars().count();
  let formatted_pinyin = unsafe {
    // TODO: The below depends on the fact that a byte representing an ASCII character (space) can't
    // appear inside a multi-byte UTF-8 character. There's probably a less hacky way to do this.
    let mut bytes_seen = 0;
    for b in rest.bytes() {
      if b == ' ' as u8 {
        syllables -= 1;
        if syllables <= 0 {
          break;
        }
      }
      bytes_seen += 1;
    }
    rest.slice_unchecked(0, bytes_seen)
  };
  rv.pinyin = cedict::pinyin_to_ascii(&formatted_pinyin);
  Ok(rv)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jilupian_line_parses_correctly() {
    let line = "紀錄片 [纪录-]     jì lù piàn       newsreel • documentary (film or TV program) • CL: 部 (bù)";
    let parsed_line = parse_line(line).unwrap();
    assert_eq!(parsed_line.trad, "紀錄片");
    assert_eq!(parsed_line.simp, "纪录片");
    assert_eq!(parsed_line.pinyin, "ji4 lu4 pian4");
  }

  #[test]
  fn cu_line_parses_correctly() {
    // tests case where trad == simp, and there is exactly one space between records
    let line = "粗 cū coarse • rough • thick (for cylindrical objects) • unfinished • vulgar • rude • crude";
    let parsed_line = parse_line(line).unwrap();
    assert_eq!(parsed_line.trad, "粗");
    assert_eq!(parsed_line.simp, "粗");
    assert_eq!(parsed_line.pinyin, "cu1");
  }
}
