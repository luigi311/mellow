use core::cell::RefCell;
use gio::prelude::*;
use glib::{GString, VariantTy};
use gtk::{gio, glib};
use std::rc::Rc;

use crate::ui::album_object::AlbumOrdering;
use crate::ui::album_page::AlbumPage;
use crate::ui::albums_page::AlbumsPage;
use crate::ui::artist_object::ArtistOrdering;
use crate::ui::artist_page::ArtistPage;
use crate::ui::artists_page::ArtistsPage;
use crate::ui::song_object::SongOrdering;
use crate::ui::songs_page::SongsPage;

#[inline]
pub fn songs_sort_mode(
    songs_page: SongsPage,
    initial_state: &GString,
) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("songs_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(initial_state.to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            songs_page.set_sort_mode(SongOrdering::from_str(&variant.get::<String>().unwrap()));
        })
        .build()
}
#[inline]
pub fn albums_sort_mode(
    albums_page: AlbumsPage,
    initial_state: &GString,
) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("albums_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(initial_state.to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            albums_page.set_sort_mode(AlbumOrdering::from_str(&variant.get::<String>().unwrap()));
        })
        .build()
}
#[inline]
pub fn artists_sort_mode(
    artists_page: ArtistsPage,
    initial_state: &GString,
) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("artists_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(initial_state.to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            artists_page.set_sort_mode(ArtistOrdering::from_str(&variant.get::<String>().unwrap()));
        })
        .build()
}

#[inline]
pub fn artists_play_mode(artists_page: ArtistsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("artists_play_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(match artists_page.get_shuffle() {
            false => "Sequential".to_variant(),
            true => "Shuffled".to_variant(),
        })
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            artists_page.set_shuffle(match &*variant.get::<String>().unwrap() {
                "Sequential" => false,
                "Shuffled" => true,
                _ => unimplemented!(),
            });
        })
        .build()
}
#[inline]
pub fn albums_play_mode(albums_page: AlbumsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("albums_play_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(match albums_page.get_shuffle() {
            false => "Sequential".to_variant(),
            true => "Shuffled".to_variant(),
        })
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            albums_page.set_shuffle(match &*variant.get::<String>().unwrap() {
                "Sequential" => false,
                "Shuffled" => true,
                _ => unimplemented!(),
            });
        })
        .build()
}
#[inline]
pub fn songs_play_mode(songs_page: SongsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("songs_play_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state(match songs_page.get_shuffle() {
            false => "Sequential".to_variant(),
            true => "Shuffled".to_variant(),
        })
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            songs_page.set_shuffle(match &*variant.get::<String>().unwrap() {
                "Sequential" => false,
                "Shuffled" => true,
                _ => unimplemented!(),
            });
        })
        .build()
}

#[inline]
pub fn artist_page_play_mode(
    artist_pages: Rc<RefCell<Vec<ArtistPage>>>,
) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("artist_page_play_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state("Sequential".to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            artist_pages.borrow().last().inspect(|album_page| {
                album_page.set_shuffle(match &*variant.get::<String>().unwrap() {
                    "Sequential" => false,
                    "Shuffled" => true,
                    _ => unimplemented!(),
                });
            });
        })
        .build()
}
#[inline]
pub fn album_page_play_mode(
    album_pages: Rc<RefCell<Vec<AlbumPage>>>,
) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("album_page_play_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state("Sequential".to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            album_pages.borrow().last().inspect(|album_page| {
                album_page.set_shuffle(match &*variant.get::<String>().unwrap() {
                    "Sequential" => false,
                    "Shuffled" => true,
                    _ => unimplemented!(),
                });
            });
        })
        .build()
}
