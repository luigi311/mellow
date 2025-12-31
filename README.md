<h1>
<p align="center">
  <img height=128 src="data/resources/com.github.userwithaname.Mellow.png">
  <br>Mellow
</h1>
  <p align="center">
    Listen to music without distraction
  </p>
</p>

# About

Mellow is an experimental music player, which strives for maximal immersion with
minimal distraction. Elements of the interface are purposefully abstracted away,
bringing music to the center of your focus.

# Roadmap

> [!CAUTION]
> This software is still in early development. It may be buggy or missing crucial
> features, and may or may not maintain backward/forward compatibility going
> forward. See [QA.md](QA.md) and [TODO.md](TODO.md) for more information.

|  #  | Feature                                   | Status  |
| :-: | ----------------------------------------- | :-----: |
|  1  | Playback & controls: play/pause/skip/seek | ✅ Done |
|  2  | Playback modes: normal/shuffle/repeat     | ✅ Done |
|  3  | Open files/folders to create a queue      | ✅ Done |
|  4  | Song queue interface & management         | ⚠️ WIP  |
|  5  | Music library                             | ⚠️ WIP: No UI yet |
|  6  | Music library search/filters              | ⚠️ WIP: Simple search, no UI yet |
|  7  | Play counts and ratings                   | ⚠️ WIP: Basic play counting, no UI yet |
|  8  | Tag-like playlists                        | ❌ TODO |
|  9  | Media controller support (MPRIS)          | ❌ TODO |
| 10  | Adaptive background/colors                | ❌ TODO |

# Building from source

## Prerequisites

### [Compile the Mellow GSchema](https://gtk-rs.org/gtk4-rs/stable/latest/book/settings.html):

On Linux or macOS:

```bash
mkdir -p $HOME/.local/share/glib-2.0/schemas
cp data/resources/com.github.userwithaname.Mellow.gschema.xml $HOME/.local/share/glib-2.0/schemas/
glib-compile-schemas $HOME/.local/share/glib-2.0/schemas/
```

Or on Windows:

```bash
mkdir C:/ProgramData/glib-2.0/schemas/
cp data/resources/com.github.userwithaname.Mellow.gschema.xml C:/ProgramData/glib-2.0/schemas/
glib-compile-schemas C:/ProgramData/glib-2.0/schemas/
```

## Build dependencies

> [!NOTE]
> DNF commands are meant for Fedora

### [GStreamer](https://gstreamer.freedesktop.org/documentation/installing/on-linux.html):
```bash
dnf install \
  gstreamer1-devel \
  gstreamer1-doc \
  gstreamer1-plugins-bad-free \
  gstreamer1-plugins-bad-free-devel \
  gstreamer1-plugins-bad-free-extras \
  gstreamer1-plugins-base-devel \
  gstreamer1-plugins-base-tools \
  gstreamer1-plugins-good \
  gstreamer1-plugins-good-extras
```

### [GTK](https://gtk-rs.org/gtk4-rs/stable/latest/book/project_setup.html)/[Libadwaita](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html):
```bash
dnf install gtk4-devel libadwaita-devel
```

### [Rust & Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

### [Build using Cargo](https://doc.rust-lang.org/cargo/commands/cargo-build.html):

```bash
cargo build --release
```

To run it, launch the executable binary in `target/release/mellow`
