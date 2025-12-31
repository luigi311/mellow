Misc:

- [-] Load song info without blocking the UI
  - TODO: Library songs/albums/artists

Main Player:

- [ ] Marquee long titles
- [ ] Background color(s) matching the album cover
  - [ ] Bonus: alter the colors differently for dark/light theme

Song queue:

- [-] Display the song queue
  - TODO: Draw the entire queue (using a more performant approach)
  - FIX: Memory leak when toggling shuffle
- [-] Allow reordering the queue
> Untested, needs UI
- [-] Allow adding songs to the queue
> Implemented, needs library UI
- [-] Allow removing items from the queue
> Needs UI for removing stoppers
- [x] Allow inserting stoppers (scheduled pause)
- [x] Access song lyrics using a header-bar icon
- [ ] Ability to select/remove multiple items at once
- [ ] Drag file/folder onto player to start a queue with them
> The bottom sheet could open automatically and focus the song
queue, then show a "Drop here to start playing" message

Music library:

- [x] Save/load user settings (such as library directories)
- [x] Serialize the library to disk
- [-] Incremental library rebuilding
  - TODO: Remove missing songs
  - TODO: Detect modifications
  - TODO: Allow users to initiate a full rebuild
- [ ] Songs page
  - [x] Buttons to play all songs (shuffled/sequential)
    - IDEA: Respect current filters (when search is implemented)
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
  - [ ] Search/filtering
- [ ] Albums page
  - [x] Buttons to play all albums (shuffled/sequential)
    - IDEA: Respect current filters (when search is implemented)
  - [ ] Show all albums
  - [ ] Album subpage, accessed from each item
    - [ ] Display album info (album cover/title/artist/year)
    - [ ] Interactive rating widget (or non-interactive average?)
    - [ ] Display a track list, each opening a Song subpage
    - [ ] Ability to set tags
    - [ ] Go to artist button
  - [ ] Search/filtering
- [ ] Artists page
  - [x] Buttons to play all artists (shuffled/sequential)
    - IDEA: Respect current filters (when search is implemented)
  - [ ] Show all artists
  - [ ] Artist subpage, accessed from each item
    - [ ] Display artist info (name/number of albums/average rating)
    - [ ] Display all albums by the artist, each opening an Album subpage
  - [ ] Search/filtering
- [x] Play counting
> Works, but the counting logic could be improved
- [ ] More complex filtering
  - [ ] Filter by tags, ratings, year, etc
  - [ ] Conditional: year < 2000, rating > 3, play-count = 0, etc
