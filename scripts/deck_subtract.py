#!/usr/bin/python3
"""
Usage: deck_subtract.py deck_a deck_b deck_out

Writes to deck_out a deck that has all the cards in deck_a that do not appear in
deck_b (based on sfld). Media files are ignored; only the cards themselves are
copied into deck_out. The deck title, etc. are copied from deck_a.
"""

import itertools
import os.path
import sqlite3
import sys
import tempfile
import zipfile

deck_a, deck_b, deck_out = sys.argv[1:]

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

b_sflds = get_sflds(deck_b)
a_dir = extract(deck_a)

conn = sqlite3.connect(os.path.join(a_dir, 'collection.anki2'))
cur = conn.cursor()
for sfld in b_sflds:
  cur.execute('delete from notes where sfld = ?', (sfld,))
cur.execute('delete from cards where nid not in (select n.id from notes n)')
conn.commit()
conn.close()

out_zip = zipfile.ZipFile(deck_out, 'w')
with open(os.path.join(a_dir, 'collection.anki2'), 'rb') as h:
  out_zip.writestr('collection.anki2', h.read())
out_zip.writestr('media', '{}\n')
out_zip.close()
