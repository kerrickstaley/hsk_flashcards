#![plugin(regex_macros)]

use chinese_note;
use cedict;
use preferred_entry;

#[derive(Clone)]
struct HskWord {
  simp: String,
  part_of_speech: String,  // usually ""
  level: u32,
}

fn get_hsk_words() -> Vec<HskWord> {
  let wordlist = include_str!("hsk_wordlist.csv");
  let re = regex!(r"(.+?),(.*?),(\d)\n");
  let mut rv = Vec::new();
  for cap in re.captures_iter(wordlist) {
    rv.push(
        HskWord{simp: cap.at(1).unwrap_or("").to_string(),
                part_of_speech: cap.at(2).unwrap_or("").to_string(),
                level: cap.at(3).unwrap_or("0").parse().unwrap_or(0)});
  }
  rv
}

pub fn get_chinese_notes() -> Vec<chinese_note::ChineseNote<'static>> {
  let dict = cedict::Dict::new_with_extra_entries(include_str!("extra_dict.txt"));
  let preferred = preferred_entry::PreferredEntryGetter::new(&dict);
  let hsk_words = get_hsk_words();

  let mut rv = Vec::new();

  for word in hsk_words {
    if word.simp == "纪录" {
      // This word is just a variant of 记录, which is already
      // in the HSK word list. Skip it.
      continue;
    }
    if dict.search_simp(&word.simp).len() == 0 {
      println!("{} not in dict", word.simp);
      continue;
    }
    let ce = preferred.get(
        &word.simp,
        if word.part_of_speech == "" { None } else { Some(&word.part_of_speech) });
    rv.push(chinese_note::ChineseNote{ce: ce,
                                      tags: vec!(format!("HSK_Level_{}", word.level))});
  }
  rv
}
