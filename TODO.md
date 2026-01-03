Song queue:

- [-] Display the song queue
  - TODO: Draw the entire queue (using a more performant approach)
  - FIX: Memory leak when toggling shuffle
- [-] Allow reordering the queue
> Untested, needs UI
- [-] Allow adding songs to the queue
> Implemented, needs library UI
- [x] Allow removing items from the queue
- [x] Allow inserting stoppers (scheduled pause)
- [x] Access song lyrics using a header-bar icon
- [ ] Ability to select/remove multiple items at once
- [x] Display a landing page
> The "Open from Disk" picker could be improved to accept directories as well
- [x] Drag file/folder onto player to start a queue with them
  - TODO: Add visual feedback when the file is over the window
  > The bottom sheet could open automatically and focus the song
  queue, then show a "Drop here to start playing" message, or the
  window could show an overlay (like Amberol does)

Music library:

- [x] Save/load user settings (such as library directories)
- [x] Serialize the library to disk
- [-] Incremental library rebuilding
  - TODO: Detect modifications
  - TODO: Allow users to initiate a full rebuild
- [ ] Songs page
  - [x] Buttons to play all songs (shuffled/sequential)
  - [ ] Show all songs
  - [ ] Song subpage, accessed from each item
    - [ ] Display song info (title/album/artist, maybe album cover)
    - [ ] Interactive rating widget
    - [ ] Play Now: Start a new queue and skip to the selected track
      - IDEA: Use a closure so the page can be reused in different contexts
    - [ ] Play Next: Insert the song to the next position in the queue
    - [ ] Ability to set tags
    - [ ] Go to album button
    - [ ] Go to artist button
- [ ] Albums page
  - [x] Buttons to play all albums (shuffled/sequential)
  - [ ] Show all albums
  - [ ] Album subpage, accessed from each item
    - [ ] Display album info (album cover/title/artist/year)
    - [ ] Interactive rating widget (or non-interactive average?)
    - [ ] Display a track list, each opening a Song subpage
    - [ ] Ability to set tags
    - [ ] Go to artist button
- [ ] Artists page
  - [x] Buttons to play all artists (shuffled/sequential)
  - [ ] Show all artists
  - [ ] Artist subpage, accessed from each item
    - [ ] Display artist info (name/number of albums/average rating)
    - [ ] Display all albums by the artist, each opening an Album subpage
- [x] Play counting
> Works, but the counting logic could be improved
- [-] Search/filtering for songs/albums/artists pages
  - FIX: Cancelling search using escape key does not un-toggle the search button
  - FIX: Cannot drag-select text because it drags the header bar
  - FIX: Spacebar-to-play shortcut interferes with library search
- [ ] More complex filtering
  - [ ] Filter by tags, ratings, year, etc
  - [ ] Conditional: year < 2000, rating > 3, play-count = 0, etc

Misc:

- [-] Load song info without blocking the UI
  - TODO: Library songs/albums/artists

Improvements:

- [ ] Marquee long titles
- [ ] Background color(s) matching the album cover
  - [ ] Bonus: alter the colors differently for dark/light theme
