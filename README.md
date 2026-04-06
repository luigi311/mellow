<h1>
<p align="center">
  <img height=128 src="data/icons/io.github.userwithaname.Mellow.png">
  <br>Mellow
</h1>
  <p align="center">
    Listen to music without distraction
  </p>
</p>

# About

Mellow is an experimental music player, which strives for maximal immersion with
minimal distraction. Elements of the interface are purposefully abstracted away,
letting music be the central point of focus.

> [!CAUTION]
> This software is in active development. Certain features may be missing, buggy, or
> incomplete. See the [Roadmap](#roadmap), [QA.md](QA.md), and [TODO.md](TODO.md) for
> more details.

# Philosophy

Mellow's primary design goal is to minimize distraction and maximize immersion,
and with that encourage experiencing the music in-the-moment.

Unlike most players, Mellow puts the currently playing song at the base of the
interface. This means that there is no "back" button on the main player, which
might be tempting to press. Rather, everything that is unrelated to the currently
playing song is done inside an overlay, hidden from view except when needed.

This divides the interface into two parts; the main player (the "now"), and the
overlay (the "not now"). The main player features the currently playing song and
player controls, and the overlay is used for everything else; browsing the library
to find what to play, editing the song queue to choose what plays next, setting the
shuffle and repeat modes, and configuring the application.
When the overlay is closed, it is time to enjoy the music.

# Features

- ✨ Sleek and minimal interface - less for the eyes, more for the ears
- 🌈 Adaptive colors - interface colors adapt to match the album cover
- 🌊 Gapless playback - enjoy stutter-free transitions between songs
- 📋 Song queue - view and edit the list of playing songs, or schedule a pause
- 📀 Music library - browse and play your local music collection
- 📂 File discovery - detects changed, moved, removed, or added song files
- 💾 Removable drives - ratings can be accessed once the library is available again
- 🪽 Fast & lightweight - responsive and quick to start, even with large libraries

# Roadmap

|  #  | Feature                                   | Status  |
| :-: | ----------------------------------------- | :-----: |
|  1  | Playback & controls: play/pause/skip/seek | ✅ Done |
|  2  | Playback modes: normal/shuffle/repeat     | ✅ Done |
|  3  | Open files/folders to create a queue      | ✅ Done |
|  4  | Song queue interface & management         | ⚠️ Possible UI element scaling issues |
|  5  | Music library                             | ⚠️ Mostly done, but see below |
| 5.1 | Artists page                              | ⚠️ Possible UI element scaling issues in subpages |
| 5.2 | Albums page                               | ⚠️ Possible UI element scaling issues |
| 5.3 | Songs page                                | ⚠️ Possible UI element scaling issues |
| 5.4 | Search/filter/sort                        | ⚠️ No conditional filtering yet |
| 5.5 | Play counts and ratings                   | ⚠️ Play counts are not shown anywhere yet |
| 5.6 | User-assigned tags                        | ❌ TODO |
|  6  | D-Bus media integration (MPRIS)           | ❌ TODO |
|  7  | Adaptive background/colors                | ✅ Done |

# Building from source

> [!NOTE]
> The below instructions are meant for Fedora;
> they might be different on other systems

## Build dependencies

### [GStreamer](https://gstreamer.freedesktop.org/documentation/installing/on-linux.html):

```bash
dnf install gstreamer1-devel

```

Recommended plugins (required to play music):
```bash
dnf install \
  gstreamer1-plugins-bad-free \
  gstreamer1-plugins-bad-free-extras \
  gstreamer1-plugins-good \
  gstreamer1-plugins-good-extras \
  gstreamer1-plugin-libav
```

### [GTK](https://gtk-rs.org/gtk4-rs/stable/latest/book/project_setup.html)/[Libadwaita](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html):
```bash
dnf install gtk4-devel libadwaita-devel
```

### [Rust & Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### [Meson](https://mesonbuild.com/SimpleStart.html#installing-meson):
```bash
dnf install meson
```

> [!TIP]
> It is also possible to build Mellow using [Cargo](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
> directly by adding `--feature no-meson`. Note that this will require manually
> [installing the GSchema](https://gtk-rs.org/gtk4-rs/stable/latest/book/settings.html),
> setting up icons, and creating the application shortcut, so Meson should be
> preferred whenever possible.

## Building and installing

### [Build using Meson](https://gtk-rs.org/gtk4-rs/stable/latest/book/meson.html#building-and-running):

Clone the source code and run the following command to build and
install Mellow on your system:

```bash
meson setup builddir --prefix=~/.local && meson install -C builddir
```

The following files and directories will be created:
```
~
├── .cache
│   └── mellow ⟵╮
│       └── …  ⟵┤
├── .config     ├─ Created when launched
│   └── mellow ⟵┤
│       └── …  ⟵╯
└── .local
    ├── bin
    │   └── mellow ⟵─ Main program executable
    └── share
        ├── applications
        │   └── io.github.userwithaname.Mellow.desktop
        ├── dbus-1
        │   └── services
        │       └── io.github.userwithaname.Mellow.service
        ├── glib-2.0
        │   └── schemas
        │       ├── io.github.userwithaname.Mellow.gschema.xml
        │       └── gschemas.compiled ⟵╮
        │           Note: May also contain schemas for other apps
        ├── icons
        │   └── hicolor
        │       └── scalable
        │           └── apps
        │               └── io.github.userwithaname.Mellow.png
        └── mellow
            └── resources.gresource
```

> [!TIP]
> Ensure the `mellow` executable is within your `$PATH` for the shortcut to
> work correctly. If you've used a different build command, the executable
> might be in a different location than shown above.
