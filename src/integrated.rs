extern crate csv;

use chinese_note;
use cedict;
use preferred_entry;

pub fn get_chinese_notes() -> Vec<chinese_note::ChineseNote<'static>> {
  let dict = cedict::Dict::new_with_extra_entries(include_str!("extra_dict_integrated.txt"));
  let preferred = preferred_entry::PreferredEntryGetter::new(&dict);
  let mut rdr = csv::Reader::from_string(include_str!("integrated_wordlist.csv"))
                .has_headers(false);
  let mut rv = Vec::new();
  for row in rdr.decode() {
    let (simp, level, lesson): (String, u32, u32) = row.unwrap();
    if dict.search_simp(&simp).len() == 0 {
      println!("{} not in dict", simp);
      continue;
    }
    let ce = preferred.get(&simp, None);
    rv.push(chinese_note::ChineseNote{
        ce: ce,
        tags: vec!(format!("IC_{}_{}", level, lesson)),
    });
  }
  rv
}
