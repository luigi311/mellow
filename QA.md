# Quality Control

Ensure the following:

Playback:

- [x] Pause/play/skip work as expected
- [x] Shuffle/repeat/sequential modes work as expected
- [ ] Seeking works as expected
  - FIX: Seeking to the end very quickly ends up skipping songs
- [x] Gapless/non-gapless playback works as expected
- [x] Non-fatal errors are handled gracefully

Interface:

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
