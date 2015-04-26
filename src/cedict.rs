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

pub fn parse_dict<'a>(dict: &'a str) -> Vec<Entry<'a>> {
  let mut rv = Vec::new();
  for line in dict.split("\n") {
    let entry_re = regex!(r"(.+?) (.+?) \[(.+?)\] /(.+)/");
    match entry_re.captures(line) {
      Some(cap) => {
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
        rv.push(
            Entry{trad: cap.at(1).unwrap_or(""),
                  simp: cap.at(2).unwrap_or(""),
                  pinyin: cap.at(3).unwrap_or(""),
                  tw_pinyin: tw_pinyin,
                  defs: defs,
                  clfrs: clfrs});
      },
      None => (),
    }
  }
  rv
}

pub fn get_dict_index<'a>(ccedict : &'a Vec<Entry<'a>>) -> HashMap<String, Vec<&'a Entry<'a>>> {
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
