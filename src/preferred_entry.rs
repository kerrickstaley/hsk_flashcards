extern crate yaml;
use cedict;
use std::collections::HashMap;
use yaml::constructor::*;

#[derive(Clone)]
struct PreferredEntry {
  pinyin: String,
  trad: String,
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


fn best_entry<'a>(simp: &str,
                  part_of_speech: Option<&str>,
                  dict: &cedict::Dict<'a>,
                  preferred: &HashMap<String, PreferredEntry>)
                  -> cedict::Entry<'a> {
  let entries = dict.search_simp(&simp);
  let mut matches = 0;
  let key = match part_of_speech {
    Some(s) => simp.to_string() + " " + s,
    None => simp.to_string(),
  };
  for entry in &entries {
    match preferred.get(&key) {
      Some(p) => {
        if (p.pinyin == "" || p.pinyin == entry.pinyin)
            && (p.trad == "" || p.trad == entry.trad) {
          return entry.clone();
        }
      },
      _ => ()
    }
    if is_good(&entry) {
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
    for entry in &entries {
      if is_good(&entry) {
        rv = entry.clone();
      }
    }
  }

  if rv.defs[0].starts_with("erhua variant of ") {
    let mut actual_simp = "".to_string();
    // TODO: this is terrible, just terrible. Tixif!
    let mut prev = '\0';
    for c in simp.chars() {
      if prev != '\0' {
        actual_simp.push(prev);
      }
      prev = c;
    }
    let actual_word = best_entry(&actual_simp, part_of_speech, dict, preferred);
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

pub struct PreferredEntryGetter<'d, 'e: 'd> {
  map: HashMap<String, PreferredEntry>,
  dict: &'d cedict::Dict<'e>,
}

// TODO: move code above into this impl
impl<'d, 'e> PreferredEntryGetter<'d, 'e> {
  pub fn new(dict: &'d cedict::Dict<'e>) -> PreferredEntryGetter<'d, 'e> {
    PreferredEntryGetter {
      map: get_preferred_entry_map(),
      dict: dict,
    }
  }

  pub fn get(&self, simp: &str, part_of_speech: Option<&str>)
      -> cedict::Entry<'e> {
    best_entry(simp, part_of_speech, &self.dict, &self.map)
  }
}
