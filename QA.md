# Quality Assurance

1. Run `cargo test` and ensure everything passes
2. Uncheck all boxes and re-test to ensure the following:

Playback:

- [x] Pause/play/skip work as expected
- [x] Shuffle/repeat/sequential modes work as expected
- [x] Seeking works as expected
  - [x] Seeking to any point in the song (click or drag)
  - [x] Seeking to the end and releasing the seek bar
  - [x] Seeking to the end and back
- [x] Gapless/non-gapless playback works as expected
- [x] Non-fatal errors are handled gracefully

Song Queue:

- [ ] Starting a new queue works as expected
  - FIX: Starting an empty queue does not unload the currently playing song
- [x] Adding items works as expected
- [x] Removing items works as expected
- [x] Reordering the queue works as expected
  - TODO: Reorder using drag & drop
- [ ] Stoppers work and behave as expected
  - FIX: Stoppers shift when toggling shuffle mode
- [ ] Scrolling works without issues
  - FIX: Scroll position resets whenever the queue changes

Music Library:

- [x] The 'Songs' page and its subpages do not have performance issues
- [ ] The 'Albums' page and its subpages do not have performance issues
  - FIX: Artwork loading on the album subpage blocks the main thread
- [ ] The 'Artists' page and its subpages do not have performance issues
  - FIX: Artwork loading on the artist subpage blocks the main thread
- [ ] Library building works in the background and doesn't affect functionality
  - FIX: No way to cancel, simultaneous changes/rebuilds could cause incorrect results
  - FIX: Rebuilding sometimes blocks library requests(?)
- [ ] Album grouping works as expected
  - FIX: Songs with different year tags are considered to be from different albums
- [ ] Searching is quick and works as expected
  - TODO: Search results for songs/albums/artists
  - TODO: Filters for songs/albums/artists
  - FIX: Spacebar-to-play shortcut interferes with library search

User Experience:

- [x] The interface is responsive as soon as launched, without delays
  - [x] With existing library
  - [x] On fresh launch
- [x] All actions respond to user input without delay
- [x] Lengthy tasks display a progress bar without blocking the interface
- [x] All settings load properly (test with non-default values)
- [ ] Does not leak memory
  - FIX: Song queue memory leak (see TODO.md)
