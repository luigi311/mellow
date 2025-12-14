Misc:

- [ ] Load song info without blocking the UI

Main Player:

- [ ] Marquee long titles
- [ ] Background color(s) matching the album cover
  - [ ] Bonus: alter the colors differently for dark/light theme

Song queue:

- [-] Display the song queue
  - TODO: Optimize memory usage/artwork loading performance
  - TODO: Draw the entire queue (using a more performant approach)
- [-] Allow reordering the queue
> Untested, needs UI
- [-] Allow adding songs to the queue
> Implemented, needs library UI
- [-] Allow removing items from the queue
> Needs UI for removing stoppers
- [x] Allow inserting stoppers (scheduled pause)
- [x] Access song lyrics using a header-bar icon

Music library:

- [ ] Save/load user settings (such as library directories)
- [ ] Serialize the library to disk
- [ ] Incremental library rebuilding
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
  - [ ] Search/filtering
- [ ] Artists page
  - [x] Buttons play all artists (shuffled/sequential)
    - IDEA: Respect current filters (when search is implemented)
  - [ ] Show all artists
  - [ ] Artist subpage, accessed from each item
    - [ ] Display artist info (name/number of albums/average rating)
    - [ ] Display all albums by the artist, each opening an Album subpage
  - [ ] Search/filtering
- [ ] Play counting
- [ ] More complex filtering
  - [ ] Filter by tags, ratings, year, etc
  - [ ] Conditional: year < 2000, rating > 3, play-count = 0, etc
