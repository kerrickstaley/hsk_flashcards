#!/usr/bin/python3

# useful: https://gist.github.com/sartak/3921255

import argparse
import os.path
import sqlite3
import tempfile
import time
import zipfile

parser = argparse.ArgumentParser(epilog='Pinyin progres won\'t be copied if --old_p isn\'t provided')
parser.add_argument('old', help='old deck with your progress')
parser.add_argument('new', help='new deck with better cards but no progress')
parser.add_argument('out', help='deck to write output to')
parser.add_argument(
    '--int2_hack',
    action='store_true',
    help='apply a hack to get the actual sfld value, specific to the Integrated 2 Kerrick is using')
parser.add_argument(
    '--old_e',
    type=int,
    help="index (0-2) of the English card in the old deck's list of card types",
    required=True)
parser.add_argument(
    '--old_c',
    type=int,
    help="index (0-2) of the Chinese Character card in the old deck's list of card types",
    required=True)
parser.add_argument(
    '--old_p',
    type=int,
    help="index (0-2) of the Pinyin card in the old deck's list of card types",
    default=-1)

args = parser.parse_args()

def extract(deck):
  """Returns directory that deck was extracted into."""
  rv = tempfile.mkdtemp()
  zipfile.ZipFile(deck).extractall(rv)
  return rv

def get_old_deck_data(old, int2_hack=False):
  dir_ = extract(old)
  conn = sqlite3.connect(os.path.join(dir_, 'collection.anki2'))
  cur = conn.execute(
      'select notes.sfld, notes.flds, cards.ord, cards.type, cards.queue, cards.due, cards.ivl, cards.factor, '
      'cards.reps, cards.lapses from notes join cards on notes.id == cards.nid')
  rv = cur.fetchall()

  if int2_hack:
    for i in range(len(rv)):
      flds = rv[i][1]
      sfld = flds.split('\x1f')[0].split('>')[-1]
      rv[i] = (sfld,) + rv[i][1:]

  return rv

old_data = get_old_deck_data(args.old, args.int2_hack)

new_dir = extract(args.new)
conn = sqlite3.connect(os.path.join(new_dir, 'collection.anki2'))

now = int(time.time())
for row in old_data:
    sfld = row[0]
    rest = row[3:]
    # ords in my deck: 0 english, 1 simp, 2 trad, 3 pinyin
    old_ord = row[2]
    if old_ord == args.old_e:
      ords = [0]
    elif old_ord == args.old_c:
      ords = [1, 2]
    elif old_ord == args.old_p:
      ords = [3]
    else:
      continue
    for ord in ords:
      conn.execute(
          'update cards set mod = ?, type = ?, queue = ?, due = ?, ivl = ?, factor = ?, reps = ?, lapses = ? '
          'where ord = ? and nid = (select id from notes where sfld = ?)',
          (now,) + rest + (ord, sfld))

conn.commit()
conn.close()

out_zip = zipfile.ZipFile(args.out, 'w')
with open(os.path.join(new_dir, 'collection.anki2'), 'rb') as h:
  out_zip.writestr('collection.anki2', h.read())
out_zip.writestr('media', '{}\n')
out_zip.close()
