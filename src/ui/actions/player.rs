use adw::subclass::prelude::*;
use glib::clone;
use gtk::{gio, glib};

use crate::ui::Window;

#[inline]
pub fn skip_prev(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("skip_prev")
        .activate(clone!(
            #[weak(rename_to=player)]
            window.imp().main_player.imp(),
            move |_, _, _| player.handle_skip_prev()
        ))
        .build()
}
#[inline]
pub fn play_pause(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_pause")
        .activate(clone!(
            #[weak(rename_to=player)]
            window.imp().main_player.imp(),
            move |_, _, _| player.handle_play_pause()
        ))
        .build()
}
#[inline]
pub fn skip_next(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("skip_next")
        .activate(clone!(
            #[weak(rename_to=player)]
            window.imp().main_player.imp(),
            move |_, _, _| player.handle_skip_next()
        ))
        .build()
}
#[inline]
pub fn play_all_songs(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_all_songs")
        .activate(clone!(
            #[weak(rename_to=songs_page)]
            window.imp().songs_page.imp(),
            move |_, _, _| songs_page.handle_play_sequential()
        ))
        .build()
}
#[inline]
pub fn shuffle_all_songs(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("shuffle_all_songs")
        .activate(clone!(
            #[weak(rename_to=songs_page)]
            window.imp().songs_page.imp(),
            move |_, _, _| songs_page.handle_play_shuffled()
        ))
        .build()
}
#[inline]
pub fn play_visible_album(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_visible_album")
        .activate(clone!(
            #[weak(rename_to=album_page)]
            window.imp().album_page.imp(),
            move |_, _, _| album_page.handle_play_sequential()
        ))
        .build()
}
#[inline]
pub fn shuffle_visible_album(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("shuffle_visible_album")
        .activate(clone!(
            #[weak(rename_to=album_page)]
            window.imp().album_page.imp(),
            move |_, _, _| album_page.handle_play_shuffled()
        ))
        .build()
}
#[inline]
pub fn play_visible_artist(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_visible_artist")
        .activate(clone!(
            #[weak(rename_to=artist_page)]
            window.imp().artist_page.imp(),
            move |_, _, _| artist_page.handle_play_sequential()
        ))
        .build()
}
#[inline]
pub fn shuffle_visible_artist(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("shuffle_visible_artist")
        .activate(clone!(
            #[weak(rename_to=artist_page)]
            window.imp().artist_page.imp(),
            move |_, _, _| artist_page.handle_play_shuffled()
        ))
        .build()
}
#[inline]
pub fn play_all_albums(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_all_albums")
        .activate(clone!(
            #[weak(rename_to=albums_page)]
            window.imp().albums_page.imp(),
            move |_, _, _| albums_page.handle_play_sequential()
        ))
        .build()
}
#[inline]
pub fn shuffle_all_albums(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("shuffle_all_albums")
        .activate(clone!(
            #[weak(rename_to=albums_page)]
            window.imp().albums_page.imp(),
            move |_, _, _| albums_page.handle_play_shuffled()
        ))
        .build()
}
#[inline]
pub fn play_all_artists(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("play_all_artists")
        .activate(clone!(
            #[weak(rename_to=artists_page)]
            window.imp().artists_page.imp(),
            move |_, _, _| artists_page.handle_play_sequential()
        ))
        .build()
}
#[inline]
pub fn shuffle_all_artists(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("shuffle_all_artists")
        .activate(clone!(
            #[weak(rename_to=artists_page)]
            window.imp().artists_page.imp(),
            move |_, _, _| artists_page.handle_play_shuffled()
        ))
        .build()
}
