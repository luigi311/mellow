# Quality Assurance

Ensure the following:

Playback:

- [x] Pause/play/skip work as expected
- [x] Shuffle/repeat/sequential modes work as expected
- [ ] Seeking works as expected
  - FIX: Gapless: Seeking after next track is loaded sometimes breaks the player
  - FIX: Gapless: Seeking to the end and back sometimes breaks the "about-to-finish" callback
- [ ] Gapless/non-gapless playback works as expected
  - FIX: Gapless: Issues with seeking
- [x] Non-fatal errors are handled gracefully

Song Queue:

- [ ] Adding songs works as expected
- [ ] Removing items works as expected
- [ ] Reordering the queue works as expected
- [ ] Stoppers work as expected

Design:

- [x] UI adapts to window size, and looks good at all sizes
- [x] Positional awareness for off-screen elements
  - [x] Different elements do not share the same off-screen position
  > Exceptions can be made for things like submenus and tabs,
  as long as they are related

Performance:

- [x] App loads instantly - if library loading takes a long time, do it progressively
- [ ] Sensible memory usage and performance for item lists (library, queue)
  - [ ] Artwork downscaling/caching, dynamic loading/unloading
- [ ] Responsive UI - all actions respond instantly with no stutters or freezes
  - [ ] Search results show up quickly and progressively
  - [ ] Actions which require time display a progress bar and do not block the UI
  - [ ] Scrolling through long lists of items is always smooth, even if images don't
  load right away
