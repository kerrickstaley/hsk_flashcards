#![feature(plugin)]
#![plugin(regex_macros)]
extern crate crypto;
extern crate getopts;
extern crate time;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate yaml;

mod anki;
mod cedict;
mod chinese_note;
mod hanping;
mod hsk;
mod preferred_entry;

use crypto::digest::Digest;
use std::ascii::AsciiExt;
use std::collections::HashMap;
use std::io::Read;

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
  let base91_table = [
      'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
      't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
      'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
      '5', '6', '7', '8', '9', '!', '#', '$', '%', '&', '(', ')', '*', '+', ',', '-', '.', '/', ':',
      ';', '<', '=', '>', '?', '@', '[', ']', '^', '_', '`', '{', '|', '}', '~'];
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

#[cfg(test)]
#[test]
fn test_prettify_pinyin() {
  assert_eq!(
      prettify_pinyin("he1 dian3 lu:4 cha2 ba5"),
      concat!(
          "<span class=\"tone1\">hē</span>",
          " <span class=\"tone3\">diǎn</span>",
          " <span class=\"tone4\">lǜ</span>",
          " <span class=\"tone2\">chá</span>",
          " <span class=\"tone5\">ba</span>"));
}

fn make_defs_html(items: &Vec<&str>) -> String {
  // doesn't perform any escaping
  let mut rv = "<ol>".to_string();
  let mut first = true;
  for item in items {
    if item.starts_with("variant of ")
       || item.starts_with("old variant of ")
       || item.starts_with("also written ") {
      continue;
    }
    if first {
      first = false;
    } else {
      rv.push(' ');
    }
    rv = rv + "<li>" + item + "</li>";
  }
  return rv + "</ol>";
}

fn make_clfr_str(clfr: &cedict::Classifier, trad_first: bool) -> String {
 let char = if clfr.simp == clfr.trad {
   clfr.simp.to_string()
 } else {
   if trad_first {
     clfr.trad.to_string() + "|" + clfr.simp
   } else {
     clfr.simp.to_string() + "|" + clfr.trad
   }
 };
 char + "(" + &prettify_pinyin(clfr.pinyin) + ")"
}

fn print_usage(program: &str, opts: getopts::Options) {
  let brief = format!(concat!(
      "Usage: {} [options]\n\n",
      "By default, builds an Anki collection.anki2 SQLite3 database that includes\n",
      "flashcards for the HSK word list. This database is written to\n",
      "/tmp/collection.anki2."), program);
  print!("{}", opts.usage(&brief));
}

fn get_pinyin_dupes<'a, 'b>(notes: &'a Vec<chinese_note::ChineseNote<'b>>)
    -> HashMap<String, Vec<&'a cedict::Entry<'b>>> {
  // returns map of pinyin (e.g. "duo1 me5") to words with that Pinyin
  let mut rv = HashMap::<String, Vec<&'a cedict::Entry<'b>>>::new();
  for note in notes {
    let mut pinyins = vec!(note.ce.pinyin.to_ascii_lowercase());
    if note.ce.tw_pinyin != "" {
      pinyins.push(note.ce.tw_pinyin.to_ascii_lowercase());
    }
    for pinyin in pinyins {
      if rv.contains_key(&pinyin) {
        rv.get_mut(&pinyin).unwrap().push(&note.ce);
      } else {
        let v = vec!(&note.ce);
        rv.insert(pinyin, v);
      }
    }
  }
  rv
}

fn get_pinyin_dupe_string_fn<'a, 'b>(
    notes: &'a Vec<chinese_note::ChineseNote<'b>>, trad_first: bool)
    -> Box<Fn(&cedict::Entry) -> String + 'a> {
  let dupes_map = get_pinyin_dupes(&notes);
  // separate items with en spaces, to make them slightly easier to read
  let en_space = '\u{2002}';
  Box::new(move |entry| {
    let mut first = true;
    let mut rv = "".to_string();
    let mut pinyins = vec!(entry.pinyin.to_ascii_lowercase());
    if entry.tw_pinyin != "" {
      pinyins.push(entry.tw_pinyin.to_ascii_lowercase());
    }
    for pinyin in pinyins {
      if !dupes_map.contains_key(&pinyin) {
        println!("warning: {} not in dupes_map", entry.pinyin.to_ascii_lowercase());
        return rv;
      }
      for dupe in dupes_map.get(&pinyin).unwrap() {
        if **dupe == *entry { continue; }
        if !first {
          rv.push(en_space);
        }
        rv.push_str("<span class=\"nobr\">");
        if trad_first {
          rv.push_str(&dupe.trad);
        } else {
          rv.push_str(&dupe.simp)
        }
        if dupe.trad != dupe.simp {
          rv.push('|');
          if trad_first {
            rv.push_str(&dupe.simp);
          } else {
            rv.push_str(&dupe.trad);
          }
        }
        rv.push_str("</span>");
        first = false;
      }
    }
    rv
  })
}

fn main() {
  let mut opts = getopts::Options::new();
  // TODO: make this smart enough to handle all possible Hanping export formats (i.e. it shouldn't
  // matter whether the user has simp or trad as primary)
  opts.optopt(
      "", "hanping_words",
      concat!("Instead of building a deck of HSK words, use WORDLIST file exported from the ",
              "Hanping Android app. The app must be set to display entries as trad [simp] when ",
              "the export is performed."),
      "WORDLIST");
  opts.optopt(
      "", "extra_entries",
      concat!("When building the deck, use the dictionary entries in ENTRIES_FILE in addition to ",
              "the CC-CEDICT dictionary. ENTRIES_FILE must be in CC-CEDICT format. Currently ",
              "ignored unless --hanping_words is passed."),
      "ENTRIES_FILE");
  opts.optflag(
      "t", "traditional",
      concat!("Display traditional characters before simplified, and Taiwanese pronunciations ",
              "before mainland."));
  opts.optflag("h", "help", "Print this help menu");

  let args: Vec<String> = std::env::args().collect();
  let program: String = args[0].clone();
  let parsed_opts = match opts.parse(&args[1..]) {
    Ok(m) => m,
    Err(f) => { panic!(f.to_string()) },
  };
  if parsed_opts.opt_present("h") {
    print_usage(&program, opts);
    return;
  }

  let mut extra_entries = String::new();
  if parsed_opts.opt_present("extra_entries") {
    match std::fs::File::open(parsed_opts.opt_str("extra_entries").unwrap())
        .and_then(|mut f| f.read_to_string(&mut extra_entries)) {
      Ok(_) => (),
      Err(e) => {
        panic!("Could not open extra_entries, or it was not unicode: {}", e);
      },
    }
  }

  let mut hanping_words = String::new();
  let notes = if parsed_opts.opt_present("hanping_words") {
    match std::fs::File::open(parsed_opts.opt_str("hanping_words").unwrap())
        .and_then(|mut f| f.read_to_string(&mut hanping_words)) {
      Ok(_) => (),
      Err(e) => {
        panic!("Could not open hanping_words, or it was not unicode: {}", e);
      }
    }
    // TODO: this will silently drop lines in extra_entries if they're malformed
    hanping::get_chinese_notes(&hanping_words, &extra_entries)
  } else {
    hsk::get_chinese_notes()
  };
  let title = if parsed_opts.opt_present("hanping_words") {
    "Hanping"
  } else {
    "HSK"
  };
  let guid_prefix = if parsed_opts.opt_present("hanping_words") {
    "kerrick hanping"
  } else {
    "kerrick hsk"
  };
  let templates_yaml = include_str!("templates.yaml")
      .replace("CHARACTER",
               if parsed_opts.opt_present("traditional") {
                 "{{#Traditional}}{{Traditional}}|{{/Traditional}}{{Simplified}}"
               } else {
                 "{{Simplified}}{{#Traditional}}|{{Traditional}}{{/Traditional}}"
               })
      .replace("PINYIN",
               if parsed_opts.opt_present("traditional") {
                 "{{#Taiwan Pinyin}}{{Taiwan Pinyin}} | {{/Taiwan Pinyin}}{{Pinyin}}"
               } else {
                 "{{Pinyin}}{{#Taiwan Pinyin}} | {{Taiwan Pinyin}}{{/Taiwan Pinyin}}"
               });

  let apkg = anki::AnkiPackage::new(
      title, include_str!("flds.json"), &templates_yaml, include_str!("card.css"));
  let pinyin_not_hint = get_pinyin_dupe_string_fn(&notes, parsed_opts.opt_present("traditional"));

  for note in &notes {
    let trad = if note.ce.simp != note.ce.trad { note.ce.trad } else { "" };
    let note_id = apkg.add_note(
        &guid_from_str(
            &(guid_prefix.to_string()
              + " " + &note.ce.simp
              + " " + &note.ce.trad
              + " " + &note.ce.pinyin)),
        &(" ".to_string() + &note.tags.connect(" ") + " "),
        &(note.ce.simp.to_string()
            + "\x1f" + &trad
            + "\x1f" + &prettify_pinyin(note.ce.pinyin)
            + "\x1f" + &make_defs_html(&note.ce.defs)
            + "\x1f" + &note.ce.clfrs.iter()
                .map(|c| make_clfr_str(c, parsed_opts.opt_present("traditional")))
                .collect::<Vec<_>>().connect(", ")
            + "\x1f" + &prettify_pinyin(note.ce.tw_pinyin)
            + "\x1f" + &pinyin_not_hint(&note.ce)),
        &note.ce.simp);
    apkg.add_card(note_id, 0);
    if trad == "" {
      apkg.add_card(note_id, 1);
    } else {
      if parsed_opts.opt_present("traditional") {
        apkg.add_card(note_id, 2);
        apkg.add_card(note_id, 1);
      } else {
        apkg.add_card(note_id, 1);
        apkg.add_card(note_id, 2);
      }
    }
    apkg.add_card(note_id, 3);
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
