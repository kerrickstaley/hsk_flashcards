#!/usr/bin/python3
"""
Usage: deck_subtract.py deck_a deck_b deck_out

Writes to deck_out a deck that has all the cards in deck_a that do not appear in
deck_b (based on sfld). Media files are ignored; only the cards themselves are
copied into deck_out. The deck title, etc. are copied from deck_a.
"""

import argparse
import itertools
import os.path
import sqlite3
import tempfile
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument('deck_a', help='deck to subtract from')
parser.add_argument('deck_b', help='deck to subtract')
parser.add_argument('deck_out', help='deck to write output to')
parser.add_argument(
    '--int2_hack',
    action='store_true',
    help='apply a hack specific to the Integrated 2 Kerrick is using to handle deck_a specially')
args = parser.parse_args()

def extract(deck):
  """Returns directory that deck was extracted into."""
  rv = tempfile.mkdtemp()
  zipfile.ZipFile(deck).extractall(rv)
  return rv

def get_sflds(deck):
  """Returns the set of sflds in deck."""
  dir_ = extract(deck)
  conn = sqlite3.connect(os.path.join(dir_, 'collection.anki2'))
  cur = conn.execute("select sfld from notes;")
  return set(itertools.chain.from_iterable(cur.fetchall()))

b_sflds = get_sflds(args.deck_b)
a_dir = extract(args.deck_a)

conn = sqlite3.connect(os.path.join(a_dir, 'collection.anki2'))
cur = conn.cursor()
for sfld in b_sflds:
  if args.int2_hack:
    # warning: unsafe SQL injection
    cur.execute("delete from notes where flds like '%>{}\x1f%'".format(sfld))
  cur.execute('delete from notes where sfld = ?', (sfld,))

cur.execute('delete from cards where nid not in (select n.id from notes n)')
conn.commit()
conn.close()

out_zip = zipfile.ZipFile(args.deck_out, 'w')
with open(os.path.join(a_dir, 'collection.anki2'), 'rb') as h:
  out_zip.writestr('collection.anki2', h.read())
out_zip.writestr('media', '{}\n')
out_zip.close()
