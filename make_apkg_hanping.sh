#!/bin/sh
set -e
cargo run --release -- --hanping_words=hanping_words.txt --extra_entries=hanping_extra_entries.txt
echo '{}' > /tmp/media
cd /tmp
zip deck.apkg collection.anki2 media
