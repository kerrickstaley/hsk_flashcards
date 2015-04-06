#![feature(plugin)]
#![plugin(regex_macros)]
#![feature(collections)]
extern crate crypto;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate yaml;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::Path;
use crypto::digest::Digest;
use rustc_serialize::json;
use yaml::constructor::*;

struct HskWord {
  simp: String,
  part_of_speech: String,  // usually ""
  level: u32,
}

struct Classifier<'a> {
  trad: &'a str,
  simp: &'a str,
  pinyin: &'a str,
}

struct CcedictWord<'a> {
  trad: &'a str,
  simp: &'a str,
  pinyin: &'a str,
  defs: Vec<&'a str>,
  clfrs: Vec<Classifier<'a>>,
}

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
      (Some(c), None) => { return true; },
      (None, Some(c)) => { return false; },
      (None, None) => { return true; },
    }
  }
}

fn parse_dict<'a>(dict: &'a str) -> Vec<CcedictWord<'a>> {
  let mut rv = Vec::new();
  for line in dict.split("\n") {
    let entry_re = regex!(r"(.+?) (.+?) \[(.+?)\] /(.+)/");
    match entry_re.captures(line) {
      Some(cap) => {
        let mut defs: Vec<&str> = cap.at(4).unwrap_or("").split("/").collect();
        let mut clfrs = Vec::new();
        if defs.len() > 0 && starts_with(defs[defs.len() - 1], "CL:") {
          let mut pieces = defs.pop().unwrap().split(":");
          pieces.next();
          for clfr_str in pieces.next().unwrap().split(",") {
            let clfr_re = regex!(r"([^\[\|]+)(?:\|([^\[]+))?\[(.+)\]");
            match clfr_re.captures(clfr_str) {
              Some(cap) => {
                clfrs.push(
                    Classifier{
                        trad: cap.at(1).unwrap_or(""),
                        simp: cap.at(2).unwrap_or(cap.at(1).unwrap_or("")),
                        pinyin: cap.at(3).unwrap_or(""),
                    }
                );
              },
              _ => { println!("Couldn't parse {} as a classifier", clfr_str) },
            }
          }
        }
        rv.push(
            CcedictWord{trad: cap.at(1).unwrap_or(""),
                        simp: cap.at(2).unwrap_or(""),
                        pinyin: cap.at(3).unwrap_or(""),
                        defs: defs,
                        clfrs: clfrs});
      },
      None => (),
    }
  }
  rv
}

fn get_dict_index<'a>(ccedict : &'a Vec<CcedictWord<'a>>) -> HashMap<String, Vec<&'a CcedictWord<'a>>> {
  let mut rv = HashMap::new();
  for i in 0..ccedict.len() {
    let key = ccedict[i].simp;
    if !rv.contains_key(key) {
      rv.insert(key.to_string(), Vec::new());
    }
    rv.get_mut(key).unwrap().push(&ccedict[i]);
  }
  rv
}

fn is_good(entry: &CcedictWord) -> bool {
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


fn best_entry<'a>(entries: &'a Vec<&'a CcedictWord<'a>>, preferred: &HashMap<String, PreferredEntry>) -> &'a CcedictWord<'a> {
  let mut matches = 0;
  for entry in entries {
    match preferred.get(entries[0].simp) {
      Some(p) => {
        if (p.pinyin == "" || p.pinyin == entry.pinyin)
            && (p.trad == "" || p.trad == entry.trad) {
          return entry;
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

  if matches >= 1 {
    for entry in entries {
      if is_good(entry) {
        return *entry;
      }
    }
  }

  entries[0]
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
  let BASE91_TABLE = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '!', '#', '$', '%', '&', '(', ')', '*', '+', ',', '-', '.', '/', ':', ';', '<', '=', '>', '?', '@', '[', ']', '^', '_', '`', '{', '|', '}', '~'];
  let mut rv_reversed = String::with_capacity(10);
  while val > 0 {
    rv_reversed.push(BASE91_TABLE[(val % 91) as usize]);
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
        continue;
      }
      if curr == ':' {
        curr = 'ü';
      }
      if "ae".contains(curr) {
        rv.push(toned_char(curr, tone));
        toned = true;
      } else if !toned && curr == 'o' && next == 'u' {
        rv.push(toned_char(curr, tone));
        toned = true;
      } else if !toned && "aeiouú".contains(curr) && !"aeiouü".contains(next) {
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
  let mut rv = "<div class=\"defs_wrapper\">\n<ol>".to_string();
  for item in items {
    rv = rv + "\n<li>\n" + item + "\n</li>";
  }
  return rv + "\n</ol>\n</div>";
}

fn main() {
  let DECK_ID : i64 = 4760850724594777;
  let MODEL_ID : i64 = 1425274727592;
  let hsk_words = get_hsk_words();
  let mut dict = parse_dict(include_str!("cedict_1_0_ts_utf-8_mdbg.txt"));
  dict.append(&mut parse_dict(include_str!("extra_dict.txt")));
  let index = get_dict_index(&dict);
  let preferred = get_preferred_entry_map();

  // make /tmp/collection.anki2 a zero-length file
  OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open("/tmp/collection.anki2")
      .unwrap();

  let conn = rusqlite::SqliteConnection::open(&std::path::Path::new("/tmp/collection.anki2")).unwrap();
  conn.execute_batch(include_str!("apkg_schema.txt")).unwrap();
  let col_sql = include_str!("apkg_col.txt")
      .replace("CARDCSS", &json::encode(&include_str!("card.css")).unwrap());
  conn.execute_batch(&col_sql).unwrap();

  for word in hsk_words {
    if !index.contains_key(&word.simp) {
      println!("{} not in dict", word.simp);
      continue;
    }
    let ref dword = best_entry(&index[&word.simp], &preferred);
    conn.execute(
        "INSERT INTO notes VALUES(null,?,?,?,?,?,?,?,?,?,?);",
        &[
            &guid_from_str(&("kerrick hsk ".to_string() + &dword.trad)),  // guid
            &MODEL_ID,  // mid
            &0,  // mod
            &-1,  // usn
            &"".to_string(),  // tags
            &(dword.simp.to_string() + &"\x1f".to_string() + &prettify_pinyin(dword.pinyin) + &"\x1f".to_string() + &make_defs_html(&dword.defs) + &"\x1f".to_string() + &dword.trad + &"\x1f\x1f\x1f".to_string()), // flds
            &dword.trad,  // sfld
            &0,  // csum, can be ignored
            &0,  // flags
            &"".to_string(),  // data
        ]).unwrap();
    let note_id = conn.last_insert_rowid();
    for ord in 0..4 {
      conn.execute(
          "INSERT INTO cards VALUES(null,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?);",
          &[
              &note_id,  // nid
              &DECK_ID,  // did
              &ord,  // ord
              &0,  // mod
              &-1,  // usn
              &0,  // type (=0 for non-Cloze)
              &0,  // queue
              &0,  // due
              &0,  // ivl
              &0,  // factor
              &0,  // reps
              &0,  // lapses
              &0,  // left
              &0,  // odue
              &0,  // odid
              &0,  // flags
              &"".to_string(),  // data
          ]).unwrap();
    }
  }
  // Set due = id + 1
  conn.execute_batch("UPDATE cards SET due = id + 1;").unwrap();
}
