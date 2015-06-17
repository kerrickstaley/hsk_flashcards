#![plugin(regex_macros)]
extern crate crypto;
extern crate time;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate yaml;

use chinese_note;
use cedict;
use std::collections::HashMap;
use yaml::constructor::*;

pub const ID_STR: &'static str = "kerrick hsk";

#[derive(Clone)]
struct HskWord {
  simp: String,
  part_of_speech: String,  // usually ""
  level: u32,
}

#[derive(Clone)]
struct PreferredEntry {
  pinyin: String,
  trad: String,
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

fn starts_with(s: &str, prefix: &str) -> bool {
  let mut sc = s.chars();
  let mut prefixc = prefix.chars();
  loop {
    match (sc.next(), prefixc.next()) {
      (Some(c1), Some(c2)) => if c1 != c2 { return false; },
      (Some(_), None) => { return true; },
      (None, Some(_)) => { return false; },
      (None, None) => { return true; },
    }
  }
}

fn is_good(entry: &cedict::Entry) -> bool {
  let reference_re = regex!(r"^variant of |old variant of |^see [^ ]+\[[^\]]+\]$");
  if reference_re.is_match(entry.defs[0]) {
    return false;
  }
  let firstchar = entry.pinyin.chars().next().unwrap();
  !('A' <= firstchar && firstchar <= 'Z')
}

fn yaml_to_preferred_entry(y: YamlStandardData) -> PreferredEntry {
  let mapping = match y {
    YamlStandardData::YamlMapping(m) => m,
    _ => panic!("data wasn't a mapping"),
  };
  let mut rv = PreferredEntry{pinyin: "".to_string(), trad: "".to_string()};
  for (key, val) in mapping {
    let key_str = match key {
      YamlStandardData::YamlString(s) => s,
      _ => panic!("data wasn't a string"),
    };
    let val_str = match val {
      YamlStandardData::YamlString(s) => s,
      _ => panic!("data wasn't a string"),
    };
    if key_str == "pinyin" {
      rv.pinyin = val_str;
    } else if key_str == "trad" {
      rv.trad = val_str;
    }
  }
  rv
}

fn get_preferred_entry_map() -> HashMap<String, PreferredEntry> {
  // TODO: the way this function works is sorta janky, try to make it cleaner
  let mut rv = HashMap::new();
  let in_str = include_str!("preferred_entries.yaml");
  let yaml_docs = yaml::parse_bytes_utf8(in_str.as_bytes()).unwrap();
  // There's only one doc, but we want to get an owned copy of it, so we can't
  // do yaml_docs[0]. Instead, we do a for-loop over yaml_docs. This moves it
  // into the for-loop, so it gets destroyed as we go along
  let mut yaml_doc = YamlStandardData::YamlNull;
  for item in yaml_docs {
    yaml_doc = item;
  }

  let yaml_vec = match yaml_doc {
    YamlStandardData::YamlMapping(v) => v,
    _ => panic!("data wasn't a mapping"),
  };

  for (key, val) in yaml_vec {
    let key_str = match key {
      YamlStandardData::YamlString(s) => s,
      _ => panic!("data wasn't a string"),
    };
    rv.insert(key_str, yaml_to_preferred_entry(val));
  }

  rv
}


fn best_entry<'a>(word: &HskWord,
                  index: &HashMap<String, Vec<&cedict::Entry<'a>>>,
                  preferred: &HashMap<String, PreferredEntry>)
                  -> cedict::Entry<'a> {
  let entries = &index[&word.simp];
  let mut matches = 0;
  let key = if word.part_of_speech == "" {
    word.simp.to_string()
  } else {
    word.simp.to_string() + " " + &word.part_of_speech
  };
  for entry in entries {
    match preferred.get(&key) {
      Some(p) => {
        if (p.pinyin == "" || p.pinyin == entry.pinyin)
            && (p.trad == "" || p.trad == entry.trad) {
          return (*entry).clone();
        }
      },
      _ => ()
    }
    if is_good(entry) {
      matches += 1;
    }
  }

  /*
  if matches > 1 {
     println!("multiple matches for {}", entries[0].simp);
  } else if matches == 0 {
    println!("no good matches for {}", entries[0].simp);
  }
  */

  let mut rv = entries[0].clone();

  if matches >= 1 {
    for entry in entries {
      if is_good(entry) {
        rv = (*entry).clone();
      }
    }
  }

  if starts_with(rv.defs[0], "erhua variant of ") {
    let mut actual_simp = "".to_string();
    // TODO: this is terrible, just terrible. Tixif!
    let mut prev = '\0';
    for c in word.simp.chars() {
      if prev != '\0' {
        actual_simp.push(prev);
      }
      prev = c;
    }
    let actual_word = best_entry(
        &HskWord{
            simp: actual_simp,
            part_of_speech: word.part_of_speech.to_string(), // TODO: why is this needed?
            level: word.level},
        index,
        preferred);
    rv = cedict::Entry{
        trad: rv.trad,
        simp: rv.simp,
        pinyin: rv.pinyin,
        tw_pinyin: actual_word.tw_pinyin.clone(),
        defs: actual_word.defs.clone(),
        clfrs: actual_word.clfrs.clone()};
  }

  rv
}

pub fn get_chinese_notes() -> Vec<chinese_note::ChineseNote<'static>> {
  let hsk_words = get_hsk_words();
  let mut dict = cedict::parse_dict(include_str!("cedict_1_0_ts_utf-8_mdbg.txt"));
  dict.append(&mut cedict::parse_dict(include_str!("extra_dict.txt")));
  let index = cedict::get_dict_index(&dict);
  let preferred = get_preferred_entry_map();

  let mut rv = Vec::new();

  for word in hsk_words {
    if word.simp == "纪录" {
      // This word is just a variant of 记录, which is already
      // in the HSK word list. Skip it.
      continue;
    }
    if !index.contains_key(&word.simp) {
      println!("{} not in dict", word.simp);
      continue;
    }
    let mut ce = best_entry(&word, &index, &preferred).clone();
    if ce.simp == ce.trad {
      ce.trad = "";
    }
    rv.push(chinese_note::ChineseNote{ce: ce,
                                      tags: vec!(format!(" HSK_Level_{} ", word.level))});
  }
  rv
}
