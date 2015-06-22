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
      // 乾 干 [Gan1] /surname Gan/
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
  rv.pinyin = pinyin_to_ascii(&formatted_pinyin);
  Ok(rv)
}

// TODO: can we make this only pub for testing?
pub fn pinyin_to_ascii(pinyin: &str) -> String {
  let data = [
      ['ā', 'á', 'ǎ', 'à', 'a'],
      ['ē', 'é', 'ě', 'è', 'e'],
      ['ī', 'í', 'ǐ', 'ì', 'i'],
      ['ō', 'ó', 'ǒ', 'ò', 'o'],
      ['ū', 'ú', 'ǔ', 'ù', 'u'],
      ['ǖ', 'ǘ', 'ǚ', 'ǜ', 'ü'],
  ];
  let mut rv = "".to_string();
  let mut tone = 5;

  'process_char: for ch in pinyin.chars() {
    if tone == 5 {
      for r in 0..6 {
        // we skip checking the last column and let this case fall-through to the below
        // the result is he same either way
        for c in 0..4 {
          if data[r][c] == ch {
            tone = (c + 1) as isize;
            if r == 5 {
              rv.push_str("u:");
            } else {
              rv.push(data[r][4]);
            }
            continue 'process_char;
          }
        }
      }
    }
    if ch == ' ' {
      rv.push_str(&tone.to_string());
      tone = 5;
    }
    rv.push(ch);
  }
  if tone != -1 {
    rv.push_str(&tone.to_string());
  }
  rv
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
  fn ouer_line_parses_correctly() {
    // tests case where tone mark is not on last vowel
    let line = "偶爾 [-尔]        ǒu ěr            occasionally • once in a while • sometimes";
    let parsed_line = parse_line(line).unwrap();
    assert_eq!(parsed_line.trad, "偶爾");
    assert_eq!(parsed_line.simp, "偶尔");
    assert_eq!(parsed_line.pinyin, "ou3 er3");
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

  #[test]
  fn test_pinyin_to_ascii() {
    assert_eq!(pinyin_to_ascii("hē diǎn lǜ chá ba"), "he1 dian3 lu:4 cha2 ba5");
  }
}
