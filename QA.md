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
- [-] Reordering the queue works as expected
  - TODO: Reorder using drag & drop
- [ ] Stoppers work and behave as expected
  - FIX: Stoppers shift when toggling shuffle mode
- [x] Scrolling works without issues

Music Library:

- [ ] The 'Songs' page and its subpages do not have performance issues
  - FIX: Performance issues while scrolling at large window sizes
- [ ] The 'Albums' page and its subpages do not have performance issues
  - FIX: Artwork loading on the album subpage blocks the main thread
  - FIX: Performance issues while scrolling at large window sizes
- [ ] The 'Artists' page and its subpages do not have performance issues
  - FIX: Artwork loading on the artist subpage blocks the main thread
- [ ] Library building works in the background and doesn't affect functionality
  - FIX: Rebuilding sometimes blocks library requests(?)
- [x] Album grouping works as expected
- [x] Searching is quick and works as expected
- [ ] Filters and sort modes work as expected
  - TODO: Implement filters

User Experience:

- [x] The interface is responsive as soon as launched, without delays
  - [x] With existing library
  - [x] On fresh launch
- [x] All actions respond to user input without delay
- [x] Lengthy tasks display a progress bar without blocking the interface
- [x] All settings load properly (test with non-default values)
- [ ] Does not leak memory
  - FIX: Song queue memory leak (see TODO.md)
  - FIX: Memory leak related to artwork loading in songs/albums pages
- [ ] No other issues found while testing
