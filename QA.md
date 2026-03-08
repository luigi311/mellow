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

- [x] Starting a new queue works as expected
- [x] Adding items works as expected
- [x] Removing items works as expected
- [x] Reordering the queue works as expected
  - TODO: Improvement: Drag & drop should provide visual feedback
- [x] Stoppers work and behave as expected
  - TODO: Improvement: Stoppers should not shift when toggling shuffle mode
- [x] Scrolling works without issues

Music Library:

- [x] The 'Songs' page and its subpages do not have performance issues
- [x] The 'Albums' page and its subpages do not have performance issues
- [x] The 'Artists' page and its subpages do not have performance issues
- [x] Library building works in the background and doesn't affect functionality
- [x] Album grouping works as expected
- [x] Searching is quick and works as expected
- [x] Sort modes work as expected
- [ ] Filtering works as expected
  - TODO: **Implement filters**

User Experience:

- [x] The interface is responsive as soon as launched, without delays
  - [x] With existing library
  - [x] On fresh launch
- [x] All actions respond to user input without delay
- [x] Lengthy tasks display a progress bar without blocking the interface
- [x] All settings load properly (test with non-default values)
- [ ] Does not leak memory
  - Investigate: Possible issue with thumbnails/artworks not being fully unloaded;
    By repeatedly toggling the shuffle mode on the queue page, memory usage increases
    each time, but never exceeds the size of the thumbnails folder (sometimes decreases
    as well)
- [x] No other issues found while testing

Design Consistency:

- [x] Similar looking elements work the same an all places
