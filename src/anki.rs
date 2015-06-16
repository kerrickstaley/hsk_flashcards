extern crate crypto;
extern crate time;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate yaml;

use rustc_serialize::json;
use std;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use yaml::constructor::*;

const DECK_ID : i64 = 1428564061183;
const MODEL_ID : i64 = 1425274727596;

pub struct AnkiPackage {
  // TODO: make this private
  pub conn: rusqlite::SqliteConnection,
  timespec: time::Timespec,
}

impl AnkiPackage {
  // TODO: this hard-codes /tmp/collection.anki2, don't do that
  pub fn new(name: &str, flds: &str, tmpls_yaml: &str, css: &str) -> AnkiPackage {
    // make /tmp/collection.anki2 a zero-length file
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("/tmp/collection.anki2")
        .unwrap();

    let rv = AnkiPackage {
      conn: rusqlite::SqliteConnection::open(
           &std::path::Path::new("/tmp/collection.anki2")).unwrap(),
      timespec: time::get_time(),
    };
    rv.conn.execute_batch(include_str!("apkg_schema.txt")).unwrap();
    rv.conn.execute_batch(&make_col_sql(
        &name, &flds, &tmpls_yaml, &css)).unwrap();
    rv
  }

  pub fn add_note(&self, guid: &str, tags: &str, flds: &str, sfld: &str) -> i64 {
    // returns inserted note's ID
    self.conn.execute(
    "INSERT INTO notes VALUES(null,?,?,?,?,?,?,?,?,?,?);",
    &[
        &guid,
        &MODEL_ID,  // mid
        &self.timespec.sec,  // mod
        &-1,  // usn
        &tags,  // tags
        &flds,  // flds
        &sfld,  // sfld
        &0,  // csum, can be ignored
        &0,  // flags
        &"",  // data
    ]).unwrap();
    self.conn.last_insert_rowid()
  }

  pub fn add_card(&self, note_id: i64, ord: i64) {
    self.conn.execute(
        "INSERT INTO cards VALUES(null,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?);",
        &[
            &note_id,  // nid
            &DECK_ID,  // did
            &ord,  // ord
            &self.timespec.sec,  // mod
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

fn yaml_string(y: YamlStandardData) -> String {
  match y {
    YamlStandardData::YamlString(s) => s,
    _ => panic!("data wasn't a string"),
  }
}

fn make_col_sql(name: &str, flds_json: &str, tmpls_yaml: &str, css: &str) -> String {
  let mut tmpls = Vec::new();
  let yaml_doc = yaml::parse_bytes_utf8(tmpls_yaml.as_bytes())
      .unwrap()
      .pop();
  let seq = match yaml_doc {
    Some(YamlStandardData::YamlSequence(s)) => s,
    _ => panic!("data wasn't a sequence"),
  };
  let mut ord = 0;
  for item in seq {
    let map = match item {
      YamlStandardData::YamlMapping(m) => m,
      _ => panic!("data wasn't a mapping"),
    };
    let mut outmap = BTreeMap::new();
    for (key, val) in map {
      outmap.insert(yaml_string(key), json::Json::String(yaml_string(val)));
    }
    outmap.insert("bafmt".to_string(), json::Json::String("".to_string()));
    outmap.insert("bqfmt".to_string(), json::Json::String("".to_string()));
    outmap.insert("did".to_string(), json::Json::Null);
    outmap.insert("ord".to_string(), json::Json::I64(ord));
    ord += 1;
    tmpls.push(outmap);
  }

  include_str!("apkg_col.txt")
      .replace("NAME", &name)
      .replace("FLDS", &flds_json)
      .replace("TMPLS", &json::encode(&tmpls).unwrap())
      .replace("CARDCSS", &json::encode(&css).unwrap())
}

