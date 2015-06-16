#![feature(plugin)]
#![plugin(regex_macros)]
#![feature(collections)]
extern crate crypto;
extern crate time;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate yaml;

mod cedict;
mod anki;
use crypto::digest::Digest;
use std::collections::HashMap;
use yaml::constructor::*;

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

fn guid_from_str(s : &str) -> String {
  let mut sha = crypto::sha2::Sha256::new();
  sha.input_str(s);
  let mut sha_out : [u8; 32] = [0; 32];
  sha.result(&mut sha_out);

  // convert first 8 bytes to u64
  let mut val : u64 = 0;
  for i in 0..8 {
    val <<= 8;
    val += sha_out[i] as u64;
  }

  // convert to base91
  let base91_table = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '!', '#', '$', '%', '&', '(', ')', '*', '+', ',', '-', '.', '/', ':', ';', '<', '=', '>', '?', '@', '[', ']', '^', '_', '`', '{', '|', '}', '~'];
  let mut rv_reversed = String::with_capacity(10);
  while val > 0 {
    rv_reversed.push(base91_table[(val % 91) as usize]);
    val /= 91;
  }
  rv_reversed.chars().rev().collect()
}

fn toned_char(c: char, tone: usize) -> char {
  let data = [
      ['ā', 'á', 'ǎ', 'à', 'a'],
      ['ē', 'é', 'ě', 'è', 'e'],
      ['ī', 'í', 'ǐ', 'ì', 'i'],
      ['ō', 'ó', 'ǒ', 'ò', 'o'],
      ['ū', 'ú', 'ǔ', 'ù', 'u'],
      ['ǖ', 'ǘ', 'ǚ', 'ǜ', 'ü'],
  ];

  for row in data.iter() {
    if row[4] == c {
      return row[tone - 1];
    }
  }

  // shouldn't reach this point...
  println!("WTF {}", c);
  c
}

fn prettify_pinyin(s: &str) -> String {
  let mut rv = String::new();
  if s == "" {
    return rv
  }
  let mut first = true;
  for syl in s.split(" ") {
    if first {
      first = false
    } else {
      rv.push(' ')
    }

    let last_byte = syl.as_bytes()[syl.len() - 1];
    if ('1' as u8) > last_byte || ('5' as u8) < last_byte {
      rv.push_str(syl);
      continue;
    }

    // we know that syllable is ASCII
    let tone: usize = syl[syl.len() - 1..].parse::<usize>().unwrap_or(0);

    rv.push_str("<span class=\"tone");
    rv.push(last_byte as char);
    rv.push_str("\">");

    let mut toned = false;

    let mut syl_iter = syl.chars();
    // curr iterates over syl[0] to syl[syl.len() - 1], and next is the char
    // after curr
    let mut curr = syl_iter.next().unwrap();
    for next in syl_iter {
      if curr == 'u' && next == ':' {
        curr = 'ü';
        continue;
      }
      if "ae".contains(curr) {
        rv.push(toned_char(curr, tone));
        toned = true;
      } else if !toned && curr == 'o' && next == 'u' {
        rv.push(toned_char(curr, tone));
        toned = true;
      } else if !toned && "aeiouü".contains(curr) && !"aeiouü".contains(next) {
        rv.push(toned_char(curr, tone));
        toned = true;
      } else {
        rv.push(curr);
      }
      curr = next;
    }

    rv.push_str("</span>");
  }
  rv
}

fn make_defs_html(items: &Vec<&str>) -> String {
  // doesn't perform any escaping
  let mut rv = "<ol>".to_string();
  for item in items {
    if starts_with(item, "variant of ")
       || starts_with(item, "old variant of ")
       || starts_with(item, "also written ") {
      continue;
    }
    rv = rv + "<li>" + item + "</li>";
  }
  return rv + "</ol>";
}

fn make_clfr_str(clfr: &cedict::Classifier) -> String {
 let char = if clfr.simp == clfr.trad {
   clfr.simp.to_string()
 } else {
   clfr.simp.to_string() + "|" + clfr.trad
 };
 char + "(" + &prettify_pinyin(clfr.pinyin) + ")"
}


fn main() {
  let hsk_words = get_hsk_words();
  let mut dict = cedict::parse_dict(include_str!("cedict_1_0_ts_utf-8_mdbg.txt"));
  dict.append(&mut cedict::parse_dict(include_str!("extra_dict.txt")));
  let index = cedict::get_dict_index(&dict);
  let preferred = get_preferred_entry_map();

  let apkg = anki::AnkiPackage::new(
      "HSK", include_str!("flds.json"), include_str!("templates.yaml"), include_str!("card.css"));

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
    let dword = best_entry(&word, &index, &preferred);
    let trad = if dword.simp == dword.trad {
      ""
    } else {
      dword.trad
    };
    let note_id = apkg.add_note(
        &guid_from_str(
            &("kerrick hsk".to_string()
              + " " + &dword.simp
              + " " + &dword.trad
              + " " + &dword.pinyin)),
        &format!(" HSK_Level_{} ", word.level),
        &(dword.simp.to_string()
            + "\x1f" + &trad
            + "\x1f" + &prettify_pinyin(dword.pinyin)
            + "\x1f" + &make_defs_html(&dword.defs)
            + "\x1f" + &dword.clfrs.iter().map(make_clfr_str).collect::<Vec<_>>().connect(", ")
            + "\x1f" + &prettify_pinyin(dword.tw_pinyin)),
        &dword.simp);
    for ord in 0..4 {
      if ord == 2 && trad == "" {
        continue;
      }
      apkg.add_card(note_id, ord);
    }
  }
  // Set due = id + 1
  apkg.conn.execute_batch("UPDATE cards SET due = id + 1;").unwrap();

  // Kill duplicate notes: 等, 对, 过, 花 each only have one entry in CC-CEDICT
  for row in apkg.conn.prepare(
      concat!(
          "select a.id, a.sfld",
          " from notes as a join notes as b",
          " on a.flds == b.flds where a.id > b.id"))
      .unwrap().query(&[]).unwrap().map(|row| row.unwrap()) {
    let note_id : i64 = row.get(0);
    // println!("deleting {}", row.get::<String>(1));
    apkg.conn.execute("delete from cards where nid == ?", &[&note_id]).unwrap();
    apkg.conn.execute("delete from notes where id == ?", &[&note_id]).unwrap();
  }
}
