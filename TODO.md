- TODO: **Downscaled artworks & thumbnail caching**
- TODO: **Toast notifications**
  - TODO: Show a notification for adding an album disc to queue
    (instead of closing the page)
  - TODO: Queue undo prompt notifications when removing items
  - TODO: Notification for an upcoming "Pause & Close Player"
    (something like: "The player is scheduled to close shortly")
  - TODO: Show notifications for playback errors

Song queue:

- [-] Display the song queue
  - FIX: Memory leak when toggling shuffle
- [-] Allow reordering the queue
  - TODO: Reorder using drag & drop
- [ ] Multi-selection mode
  - [ ] Item selection (checkbox in place of the cover image)
  - [ ] Removing multiple items at once
  - [ ] Rating multiple items at once
- [x] Allow adding songs to the queue
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
- [x] Go to album button on the queue subpage
- [x] Go to artist button on the queue subpage

- FIX: Moved songs are not playable in the restored queue,
  even when the library is able to locate them

Music library:

- [ ] Save/load user settings and application state
  - TODO: Remember repeat mode
  - IDEA: Remember filters?
  - IDEA: Remember if sort order was reversed(?)
- [ ] Allow initiating a full library rebuild
- [-] Search/filtering for songs/albums/artists pages
  - FIX: Cannot drag-select text because it drags the header bar
- [x] Songs/albums/artists sort modes
  - [x] Date added (either newest or oldest)
  - [x] Release date (either newest or oldest)
  - [x] Best/worst rating
  - [x] Most/least played
  - [x] Default
- [ ] **Songs/albums/artists filtering**
  - TODO: Create a submenu in the sort dropdown for selecting filters
  - [ ] Filter by tags, ratings, year, etc
  - IDEA: Conditional filters: year < 2000, rating > 3, play-count = 0, etc
  > Note: may not work with the dropdown-submenu design
- [ ] Artists page
  - [x] Buttons to play all artists (shuffled/sequential)
  - [x] Show all artists
  - [-] Artist subpage, accessed from each item
    - [-] Display artist info (name/number of albums/average rating)
    - [x] Display all albums by the artist, each opening an Album subpage
    - [x] Play/Shuffle buttons
    - [x] Add to Queue button
- [ ] Albums page
  - [x] Buttons to play all albums (shuffled/sequential)
  - [x] Show all albums
  - [-] Album subpage, accessed from each item
    - [x] Display album info (album cover/title/artist/year/average rating)
    - [x] Play/Shuffle buttons
    - [x] Add to Queue button
    - [x] Display a track list, each opening a Song subpage
    - [ ] Visually separate songs from different disks
    - [ ] Tag management (user-specified album tags (inferred from songs?))
    - [x] Go to artist button
- [ ] Songs page
  - [x] Buttons to play all songs (shuffled/sequential)
  - [x] Show all songs
  - [-] Song subpage, accessed from each item
    - [-] Display song info (title/album/artist, maybe album cover)
      - TODO: Needs design improvements
    - [x] Interactive rating widget
    - [x] Play Now: Start a new queue and skip to the selected track
    - [x] Play Next: Insert the song to the next position in the queue
    - [x] Add to Queue button
    - [ ] Tag management (user-specified song tags)
    - [x] Go to album button
    - [x] Go to artist button
- [x] Play counting
> Works, but the counting logic could be improved

Ideas for improvements:

- Marquee long titles
- Ability to disable library directories(?)
> Disabled directories would still retain song data (play counts, etc),
> but be excluded from the actual `songs`/`albums`/`artists` used by the
> `Library` (design needed for enabling/disabling libraries)
- Main player:
  - Display a hamburger menu on the opposite side of the close button:
    - Move the volume widget into the menu
    - Add a rating widget
    - Move the 'About' button into the menu
    - Could also move the settings, and make it a popup window, then something
      else can be moved into that overlay tab (maybe current file details/lyrics?)
- Queue page:
  - Show a track number as well?
  - Undo (toast) for removed queue items
- Song page:
  - The library song page and queue subpage could display more information
    about the song, such as track number, disc, year, duration, play count,
    format/sample rate, filename, etc.
  - An 'Open With' or 'Show on Disk' button
- Library:
  - The library could get rid of the home page, and instead switch the pages
    using a dropdown menu in the headerbar instead (in place of the back button)
- The 'Go To Album/Artist' buttons could pop instead of pushing when the previous
  page is the same as the one that is about to open
