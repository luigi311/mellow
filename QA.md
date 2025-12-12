# Quality Assurance

Uncheck all boxes and re-test to ensure the following:

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

- [x] Adding items works as expected
- [ ] Removing items works as expected
  - FIX: Gapless: Removing next song when it is already loaded crashes the player
- [ ] Reordering the queue works as expected
- [ ] Stoppers work and behave as expected
  - FIX: Stoppers shift when toggling shuffle mode
- [ ] The song queue does not cause performance issues
  - FIX: UI hangs while the song info for queue items is being loaded

Music Library:

- [ ] The 'Songs' page and its subpages do not have performance issues
- [ ] The 'Albums' page and its subpages do not have performance issues
- [ ] The 'Artists' page and its subpages do not have performance issues

Interface:

- [x] UI adapts to window size, and looks good at all sizes
- [x] Positional awareness for off-screen elements
  - [x] Different elements do not share the same off-screen position
  > Exceptions can be made for things like submenus and tabs,
  as long as they are related

User Experience:

- [ ] All settings load properly (with non-default values)
  - TODO: Save/load library directories

Performance:

- [x] App loads instantly - if library loading takes a long time, do it progressively
- [ ] Sensible memory usage and performance for item lists (library, queue)
  - [ ] Artwork downscaling/caching, dynamic loading/unloading
- [ ] Responsive UI - all actions respond instantly with no stutters or freezes
  - [ ] Search results show up quickly and progressively
  - [ ] Actions which require time display a progress bar and do not block the UI
  - [ ] Scrolling through long lists of items is always smooth, even if images don't
  load right away
