use std::collections::HashMap;

#[derive(Clone)]
pub struct Entry<'a> {
  pub trad: &'a str,
  pub simp: &'a str,
  pub pinyin: &'a str,
  pub tw_pinyin: &'a str,
  pub defs: Vec<&'a str>,
  pub clfrs: Vec<Classifier<'a>>,
}

#[derive(Clone)]
pub struct Classifier<'a> {
  pub trad: &'a str,
  pub simp: &'a str,
  pub pinyin: &'a str,
}

pub struct DictSearchParams<'a> {
  simp: Option<&'a str>,
  trad: Option<&'a str>,
  pinyin: Option<&'a str>,
}

fn parse_entry<'a>(entry_str: &'a str) -> Option<Entry<'a>> {
  let entry_re = regex!(r"(.+?) (.+?) \[(.+?)\] /(.+)/");
  entry_re.captures(entry_str).map(|cap| {
    let mut defs: Vec<&str> = cap.at(4).unwrap_or("").split("/").collect();
    let mut clfrs = Vec::new();
    let mut tw_pinyin = "";
    let mut i = 0;
    while i < defs.len() {
      if starts_with(defs[i], "CL:") {
        let mut pieces = defs.remove(i).splitn(2, ":");
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
      } else if starts_with(defs[i], "Taiwan pr. ") {
        let tw_pinyin_re = regex!(r"^Taiwan pr\. \[([a-zA-Z0-9: ]+)\]$");
        match tw_pinyin_re.captures(defs[i]) {
          Some(cap) => {
            tw_pinyin = cap.at(1).unwrap();
            defs.remove(i);
          },
          _ => {
            // println!("Couldn't parse {} as a Taiwan pronunciation", defs[i]);
            i += 1;
          }
        }
      } else {
        i += 1;
      }
    }
    Entry{trad: cap.at(1).unwrap_or(""),
          simp: cap.at(2).unwrap_or(""),
          pinyin: cap.at(3).unwrap_or(""),
          tw_pinyin: tw_pinyin,
          defs: defs,
          clfrs: clfrs}
  })
}

fn build_index<'a, 'b, F>(entries: &'a Vec<Entry<'b>>, get_key: F)
    -> HashMap<String, Vec<usize>>
    where F : Fn(&Entry<'b>) -> &'b str {
  let mut rv = HashMap::new();
  for i in 0..entries.len() {
    let key = get_key(&entries[i]);
    if !rv.contains_key(key) {
      rv.insert(key.to_string(), Vec::new());
    }
    rv.get_mut(key).unwrap().push(i);
  }
  rv
}

fn entry_matches(entry: &Entry, params: &DictSearchParams) -> bool {
  match params.trad {
    Some(trad) => {
      if entry.trad != trad {
        return false;
      }
    },
    None => (),
  }
  match params.simp {
    Some(simp) => {
      if entry.simp != simp {
        return false;
      }
    },
    None => (),
  }
  match params.pinyin {
    Some(pinyin) => {
      if entry.pinyin != pinyin {
        return false;
      }
    },
    None => (),
  }
  true
}

pub struct Dict<'a> {
  entries: Vec<Entry<'a>>,
  trad_idx: HashMap<String, Vec<usize>>,
  simp_idx: HashMap<String, Vec<usize>>,
  pinyin_idx: HashMap<String, Vec<usize>>,
}

impl<'a> Dict<'a> {
  pub fn new() -> Dict<'static> {
    Dict::new_with_extra_entries("")
  }

  pub fn new_with_extra_entries<'b>(extra: &'b str) -> Dict<'b> {
    let mut rv = Dict {
      entries: Vec::new(),
      trad_idx: HashMap::new(),
      simp_idx: HashMap::new(),
      pinyin_idx: HashMap::new(),
    };
    // entries from "extra" will appear before entries from the main dict
    for line in extra.split("\n").chain(include_str!("cedict_1_0_ts_utf-8_mdbg.txt").split("\n")) {
      match parse_entry(line) {
        Some(ent) => {
          rv.entries.push(ent);
        },
        None => (),
      }
    }
    rv.trad_idx = build_index(&rv.entries, |ent| ent.trad);
    rv.simp_idx = build_index(&rv.entries, |ent| ent.simp);
    rv.pinyin_idx = build_index(&rv.entries, |ent| ent.pinyin);
    rv
  }

  pub fn search(&self, params: DictSearchParams) -> Vec<Entry<'a>> {
    // TODO: this is hella messy, tixif!
    let candidate_idxs = match params.trad.and_then(|x| self.trad_idx.get(x)) {
      Some(c) => c,
      None => match params.simp.and_then(|x| self.simp_idx.get(x)) {
        Some(c) => c,
        None => match params.pinyin.and_then(|x| self.pinyin_idx.get(x)) {
          Some(c) => c,
          // either there were no candidates (HashMap lookup returned None) or the caller didn't
          // fill out any of params's fields
          None => { return Vec::new(); },
        },
      },
    };
    let mut rv = Vec::new();
    for &i in candidate_idxs {
      let candidate = &self.entries[i];
      if entry_matches(&candidate, &params) {
        rv.push(candidate.clone());
      }
    }
    rv
  }

  pub fn search_simp(&self, simp: &str) -> Vec<Entry<'a>> {
    self.search(DictSearchParams{simp: Some(simp), trad: None, pinyin: None})
  }
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
