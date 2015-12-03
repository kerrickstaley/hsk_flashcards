#!/bin/sh
set -e
cargo run --release -- --integrated "$@"
echo '{}' > /tmp/media
cd /tmp
zip integrated_chinese_deck.apkg collection.anki2 media
