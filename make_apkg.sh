#!/bin/sh
set -e
cargo run --release -- "$@"
echo '{}' > /tmp/media
cd /tmp
zip hsk_deck.apkg collection.anki2 media
