#![feature(plugin)]
#![plugin(regex_macros)]
extern crate crypto;
extern crate regex;
extern crate rusqlite;
extern crate serialize;

use std::collections::HashMap;
use std::old_io::{File, Open, Write};
use crypto::digest::Digest;
use serialize::json;

struct HskWord {
  simp: String,
  part_of_speech: String,  // usually ""
  level: u32,
}

struct CcedictWord<'a> {
  trad: &'a str,
  simp: &'a str,
  pinyin: &'a str,
  defs: Vec<&'a str>,
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

fn parse_dict<'a>(dict: &'a str) -> Vec<CcedictWord<'a>> {
  let re = regex!(r"(.+?) (.+?) \[(.+?)\] /(.+?/)+?");
  let mut rv = Vec::new();
  for line in dict.split("\n") {
    match re.captures(line) {
      Some(cap) => {
        let mut defs = Vec::new();
        for i in 4..cap.len() {
          defs.push(cap.at(i).unwrap_or(""));
        }
        rv.push(
            CcedictWord{trad: cap.at(1).unwrap_or(""),
                        simp: cap.at(2).unwrap_or(""),
                        pinyin: cap.at(3).unwrap_or(""),
                        defs: defs});
      },
      None => (),
    }
  }
  rv
}

fn get_dict_index(ccedict : &Vec<CcedictWord>) -> HashMap<String, usize> {
  let mut rv = HashMap::new();
  for i in 0..ccedict.len() {
    rv.insert(ccedict[i].simp.to_string(), i);
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
    val += std::num::cast(sha_out[i]).unwrap();
  }

  // convert to base91
  let BASE91_TABLE = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '!', '#', '$', '%', '&', '(', ')', '*', '+', ',', '-', '.', '/', ':', ';', '<', '=', '>', '?', '@', '[', ']', '^', '_', '`', '{', '|', '}', '~'];
  let mut rv_reversed = String::with_capacity(10);
  while val > 0 {
    rv_reversed.push(BASE91_TABLE[(val % 91) as usize]);
    val /= 91;
  }
  rv_reversed.as_slice().chars().rev().collect()
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
    for i in 0..syl.len() - 1 {
      let mut curr = syl.char_at(i);
      let next = syl.char_at(i + 1);
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
    }

    rv.push_str("</span>");
  }
  rv
}

fn main() {
  let DECK_ID : i64 = 4760850724594777;
  let MODEL_ID : i64 = 1425274727592;
  let hsk_words = get_hsk_words();
  let mut dict = parse_dict(include_str!("cedict_1_0_ts_utf-8_mdbg.txt"));
  dict.append(&mut parse_dict(include_str!("extra_dict.txt")));
  let index = get_dict_index(&dict);

  // make /tmp/collection.anki2 a zero-length file
  File::open_mode(&Path::new("/tmp/collection.anki2"), Open, Write).unwrap().truncate(0).unwrap();

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
    let ref dword = dict[index[word.simp]];
    conn.execute(
        "INSERT INTO notes VALUES(null,?,?,?,?,?,?,?,?,?,?);",
        &[
            &guid_from_str(&("kerrick hsk ".to_string() + &dword.trad)),  // guid
            &MODEL_ID,  // mid
            &0,  // mod
            &-1,  // usn
            &"".to_string(),  // tags
            &(dword.simp.to_string() + &"\x1f".to_string() + &prettify_pinyin(dword.pinyin) + &"\x1f".to_string() + &dword.defs[0] + &"\x1f".to_string() + &dword.trad + &"\x1f\x1f\x1f".to_string()), // flds
            &dword.trad,  // sfld
            &0,  // csum, can be ignored
            &0,  // flags
            &"".to_string(),  // data
        ]);
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
          ]);
    }
  }
  // Set due = id + 1
  conn.execute_batch("UPDATE cards SET due = id + 1;");
}
