Song queue:

- FIX: Queue subpage should close (or update) when the queue changes
(for example, starting a new queue still shows old info, which leads
to misleading behavior - pressing play would jump to a different song
than shown on the page, or even try to index out of bounds)
- [-] Display the song queue
  - TODO: Draw the entire queue (using a more performant approach)
  - FIX: Memory leak when toggling shuffle
- [-] **Allow reordering the queue**
  - TODO: Reorder using drag & drop
- [ ] **Multi-selection mode**
  - [ ] Item selection (checkbox in place of the cover image)
  - [ ] Removing multiple items at once
  - [ ] Rating multiple items at once
- [-] Allow adding songs to the queue
  - TODO: Support adding albums
  - TODO: Support adding artists
- [x] Allow removing items from the queue
- [x] Allow inserting stoppers (scheduled pause)
- [x] Access song lyrics using a header-bar icon
- [x] Display a landing page
> The "Open from Disk" picker could be improved to accept directories as well
- [x] Drag file/folder onto player to start a queue with them
  - TODO: Add visual feedback when the file is over the window
  > The bottom sheet could open automatically and focus the song
  queue, then show a "Drop here to start playing" message, or the
  window could show an overlay (like Amberol does)

Music library:

- [ ] Save/load user settings (such as library directories)
  - TODO: Save/load shuffle preference for individual views
- [ ] Allow initiating a full library rebuild
- [-] **Search/filtering for songs/albums/artists pages**
  - TODO: Show the search results
  - FIX: Cannot drag-select text because it drags the header bar
  - FIX: Spacebar-to-play shortcut interferes with library search
- [ ] More complex filtering
  - [ ] Filter by tags, ratings, year, etc
  - [ ] Conditional: year < 2000, rating > 3, play-count = 0, etc
- [ ] Artists page
  - [ ] **Search query filtering**
  - [ ] Sort modes & custom filters
  - [x] Buttons to play all artists (shuffled/sequential)
    - TODO: Improve dropdown behavior
  - [-] Show all artists
    - TODO: **Show images (and/or other visual improvements)**
  - [-] Artist subpage, accessed from each item
    - [-] Display artist info (name/number of albums/average rating)
    - [x] Display all albums by the artist, each opening an Album subpage
    - [x] Play/Shuffle buttons
    - [ ] Add to Queue button
- [ ] Albums page
  - [ ] **Search query filtering**
  - [ ] Sort modes & custom filters
  - [x] Buttons to play all albums (shuffled/sequential)
    - TODO: Improve dropdown behavior
  - [-] Show all albums
    - TODO: **Show album artworks**
  - [-] Album subpage, accessed from each item
    - [x] Display album info (album cover/title/artist/year)
    - [ ] **Interactive rating widget**
      - TODO: Decide how ratings should be handled and how to store them
    - [x] Play/Shuffle buttons
    - [ ] Add to Queue button
    - [x] Display a track list, each opening a Song subpage
    - [ ] Visually separate songs from different disks
    - [ ] Ability to set tags
    - [ ] **Go to artist button**
- [ ] Songs page
  - [ ] **Search query filtering**
  - [ ] Sort modes & custom filters
  - [x] Buttons to play all songs (shuffled/sequential)
    - TODO: Improve dropdown behavior
  - [-] Show all songs
    - TODO: **Show album artworks**
  - [-] Song subpage, accessed from each item
    - [-] Display song info (title/album/artist, maybe album cover)
    - [x] Interactive rating widget
    - [x] Play Now: Start a new queue and skip to the selected track
    - [x] Play Next: Insert the song to the next position in the queue
    - [ ] Add to Queue button
    - [ ] Ability to set tags
    - [ ] **Go to album button**
    - [ ] **Go to artist button**
- [x] Play counting
> Works, but the counting logic could be improved

Misc:

- [-] Load song info without blocking the UI
  - TODO: Library songs/albums/artists

Ideas for improvements:

- Marquee long titles
- Ability to disable library directories(?)
> Disabled directories would still retain song data (play counts, etc),
> but be excluded from the actual `songs`/`albums`/`artists` used by the
> `Library` (design needed for enabling/disabling libraries)
- Queue page design improvements
  - Each row could also show duration and track number
- Song page design improvements
  - The library song page and queue subpage could display more information
    about the song, such as track number, disc, year, duration, play count,
    format/sample rate, filename, etc.
  - An 'Open With' or 'Show on Disk' button
