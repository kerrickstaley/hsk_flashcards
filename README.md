# hsk_flashcards
Program (in Rust) to generate Anki flashcards for the Hanyu Shuiping Kaoshi (a Chinese language proficiency test).

You can download the deck created by this program [here](https://ankiweb.net/shared/info/1855818143).

To run this program, you must have an up-to-date installation of Rust.

Run ```./make_apkg.sh``` from the root directory; the output file will be stored at ```/tmp/deck.apkg```.

Note: this is my very first Rust program, and was created over the course of several late nights.
As such, the code quality may be lacking.

## Publishing
If you are Kerrick, you can publish the deck by following these steps:

1. Generate the deck using ```./make_apkg.sh```.
1. Sync phone to AnkiWeb, and AnkiWeb to computer.
1. In Anki, create a backup of your collection.
1. Delete your Chinese::HSK deck.
1. Import ``/tmp/deck.apkg``.
1. In the options for the HSK deck, change "Show new cards in random order" to "Show new cards in order added" (not sure this matters).
1. Sync. (Pray.)
1. Go to https://ankiweb.net/decks/, and pick Options -> Share next to the HSK deck.
1. Edit the release notes. Give migration instructions if you added new fields.
1. Publish!
1. Import your backup (auto-deletes HSK deck).
1. Tools -> Preferences -> Network -> check "On next sync, force changes in one direction".
1. Sync and choose "Upload to AnkiWeb".
