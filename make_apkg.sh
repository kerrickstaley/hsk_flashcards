#!/bin/sh
set -e
cargo run --release -- "$@"
echo '{}' > /tmp/media
cd /tmp
zip deck.apkg collection.anki2 media
