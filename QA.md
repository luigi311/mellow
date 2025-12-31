# Quality Assurance

Uncheck all boxes and re-test to ensure the following:

Playback:

- [ ] Pause/play/skip work as expected
  - FIX: Spacebar-to-play shortcut interferes with library search
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
- [ ] Reordering the queue works as expected
- [ ] Stoppers work and behave as expected
  - FIX: Stoppers shift when toggling shuffle mode
- [ ] Scrolling works without issues
  - FIX: Scroll position resets whenever the queue changes

Music Library:

- [ ] The 'Songs' page and its subpages do not have performance issues
- [ ] The 'Albums' page and its subpages do not have performance issues
- [ ] The 'Artists' page and its subpages do not have performance issues
- [x] The player works while the library is building in the background
- [ ] Search results show up quickly or progressively

User Experience:

- [x] The interface is responsive as soon as launched, without delays
  - [x] With existing library
  - [x] On fresh launch
- [x] All actions respond to user input without delay
- [x] Lengthy tasks display a progress bar without blocking the interface
- [x] All settings load properly (test with non-default values)
