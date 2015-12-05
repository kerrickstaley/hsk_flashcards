#!/bin/bash
set -e

(
  cd ..
  ./make_apkg_integrated.sh --traditional
)

./migrate_progress.py \
  '/tmp/Chinese__Integrated Chinese Level 1 (3rd Edition).apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  --old_c 0 \
  --old_e 1

./migrate_progress.py \
  '/tmp/Chinese__Integrated Chinese Level 2 (3rd Edition).apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  --int2_hack \
  --old_e 0 \
  --old_c 1 \
  --old_p 2

./deck_subtract.py \
  '/tmp/Chinese__Integrated Chinese Level 1 (3rd Edition).apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  '/tmp/old_integrated_1_pruned.apkg'

./deck_subtract.py \
  '/tmp/Chinese__Integrated Chinese Level 2 (3rd Edition).apkg' \
  '/tmp/integrated_chinese_deck.apkg' \
  '/tmp/old_integrated_2_pruned.apkg' \
  --int2_hack
