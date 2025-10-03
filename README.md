# Build Dependencies:

> [!NOTE]
> These instructions are meant for Fedora

[Rust & Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

[GStreamer](https://gstreamer.freedesktop.org/documentation/installing/on-linux.html):
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

[GTK](https://gtk-rs.org/gtk4-rs/stable/latest/book/project_setup.html)
/[Libadwaita](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html):
```bash
dnf install gtk4-devel libadwaita-devel
```
