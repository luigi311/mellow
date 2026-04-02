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
- [ ] Removal undo works as expected
  - FIX: Undo removal in shuffle mode inserts tracks to the end of the sequential queue
    instead of the previous position
  - FIX: Toggling shuffle before pressing undo inserts at the wrong position
  - FIX: Encountering a stopper after removing an item ahead of the playing song results in undo
    re-inserting the item at the wrong position (stopper is removed, so the index is off by one)
- [ ] Reordering the queue works as expected
  - FIX: Dragging an item and dropping it after the song changes moves the wrong item if the
    dragged item was ahead of the playing song and a stopper was encountered while dragging
  - TODO: Improvement: Scroll when dragging close to the view borders
    - IDEA: Also pan if dragging onto the pan button, once panning is implemented
- [x] Selection mode works as expected
  - FIX: Selection mode should not exit when the song changes, and selections should be preserved
  - [x] Removing multiple items at once works as expected
- [x] Stoppers work and behave as expected
  - TODO: Improvement: Stoppers should not shift when toggling shuffle mode
- [x] The landing page is shown for empty queues and works without issues

Music Library:

- [x] The 'Songs' page and its subpages work as expected
- [x] The 'Albums' page and its subpages work as expected
- [x] The 'Artists' page and its subpages work as expected
- [x] Library building works in the background and doesn't affect functionality
- [ ] Searching is quick and works as expected
  - FIX: Items sometimes don't show up until scrolling after searching
  - FIX: Cannot select text because it drags the header bar (except with `no-meson`)
  - TODO: The escape key should empty the search query when focused
- [x] Sort modes work as expected
- [ ] Filtering works as expected
  - TODO: **Implement filters**

User Experience:

- [x] The interface is responsive as soon as launched, without delays
  - [x] With existing library
  - [x] On fresh launch
- [ ] All actions respond to user input without delay
  - FIX: Slight stutter when the queue is updated
- [ ] All actions provide visual feedback
  - TODO: Visual feedback for dragging files onto the player
- [x] Lengthy tasks display a progress bar without blocking the interface
- [x] All settings load properly (test with non-default values)
- [ ] Does not leak memory
  - FIX: Possible issue with thumbnails/artworks not being fully unloaded; by repeatedly
    toggling the shuffle mode on the queue page, memory usage increases each time, but
    never exceeds the size of the thumbnails folder (sometimes decreases as well)
- [x] No other issues found while testing

Design Consistency:

- [x] Similar looking elements work the same an all places
