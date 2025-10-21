Note: Document needs updating after mockup changes

# Summary:

Minimalistic music player for playing local music with a distractionless interface.
No elements or features should be added if they distract or break immersion.

The hope is to develop this application for Linux first, then expand to Android,
and later MacOS/Windows/iOS.

# Main UI:

Note: Figure out the UI before attempting the actual development.

It could go something like this:

1. Document all functionality and visuals, sketch all parts of the interface
2. Build all of the UI - all of the interface, which still does nothing
3. Implement the functionality where it is missing (the UI becomes a template)

## Now playing

A single dedicated screen for the currently playing song. Shows the album cover,
and some essential elements, but nothing else.

'Now playing' should be the default screen for the application. To access your library,
press the library button, and a new screen will appear from the bottom, overlaid on top
(see 'Bottom Sheet' in the Adwaita Demo). (The library could open automatically on launch
when the queue is empty, or alternatively, a landing page could be displayed instead of
the usual elements and controls.)

Elements (in order of priority):

- Album cover (this should take up most of the space)
- Playback controls (⏮ ⏸ ⏭)
- Seek bar (or another way to tell time in the song and navigate it)
- Background (possibly matching the album art colors in some way)
- Shuffle/repeat buttons (if those are to be supported)
- Menus (perhaps a way to access the app settings)

I think I could experiment with the design of the seek bar. All music players have it,
but I don't like that it feels a bit like a progress bar, like it's incomplete until it
fills up, rather than encouraging the music to be enjoyed 'in-the-moment'.
The purpose of the seek bar is to tell the user how far into the track they are, and let
them navigate through the track if they wish to do so; is there a better approach?

Idea: When the album ends, allow the user to rate the album. Five grey stars would appear
in-place of the album cover, and get highlighted when the user chooses a rating. The user
could also assign a 'mood' to the album, which could help with finding the right album in
to listen to in the future.

Idea: The pause-play button could change to a library button when the queue is empty

Idea: It might be worth having a way to display lyrics for the currently playing song.

Idea: The shuffle/repeat buttons could be the same button (cycling between ➡️/🔀/🔄/🔂),
which would leave room for a button to show song lyrics or a sleep timer. This may break
functional symmetry, and may be tedious to use since it has too many states...

Idea: Don't have the shuffle/repeat buttons at all; for shuffle, have a dedicated playlist
or mode. The shuffle/repeat buttons would instead be replaced with lyrics/volume.
It's just an idea, and I don't expect it to stick. Worth considering, though.


## Library

This is where you navigate your collection and choose what to play.
The app may also support features such as ratings and selecting a 'mood' or tag.
Mood could be used to help the user decide what to listen to, by filtering the
library (for example, to show only "happy" and "relaxed" albums/songs).
(Design needed)

## Settings

The app may need a settings screen, but it's even better if it's not needed. The settings
could be accessed from the library, either as a tab or using a button.
(Design needed)

The user may need to configure library locations, fetatures such as ReplayGain, and possibly
the application visuals.

# Design Ideas

## Glowing, reactive background ('now playing' screen)

- Album cover displayed over a black background
- The underside of the album cover glows with the colors of the cover to illuminate the
    background
- Realtime stereo volume information determines which parts of the glow should be brighter
    This means that quieter parts of the track will appear darker. It might make sense to
    account for the track volume by analyzing the waveform and scaling accordingly.
    Maybe ReplayGain could help with that.
- The glow uses the colors from (the edges of) the album cover
    If I do the glow as a shader effect, I could maybe determine the colors by sampling the
    pixels along a line going from the current background pixel and the album cover center,
    and take an average color, with a bias towards the outer pixels of the cover.
    Optimizations could be made by either skipping pixels (could skip more inner ones for
    example), or leaving the inner pixels out entirely (e.g. ignore pixels after reaching
    sample count limit).
    It might make sense allow some options: dynamic glow, static glow, blurred background,
    basic. It might also make sense to allow power-saving options to limit the effect
    refresh rate.
- The playback controls could also match the colors and glow while playing

## Gradient background

- Find the two dominant colors ("main" and "secondary")
- Use the secondary color as the base for the window
- Use the main color as a gradient, expanding outwards from behind the artwork
- Apply some sort of color grading or tonemapping to darken it and increase contrast
- For the light system theme, there could be some gamma or offset applied
- Could also use different tonemappers for light/dark themes

# Implementation Ideas

- Separate functionality by threads (e.g.: main/UI thread, playback thread, VFX thread, …)
- Keep things modular - isolate different functionality as much as possible to make it easier to
  change and swap out individual parts (for example, a different playback engine or UI framework)
