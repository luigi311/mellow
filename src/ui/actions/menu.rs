use gio::prelude::*;
use gst::glib::VariantTy;
use gtk::gio;

use crate::ui::album_object::AlbumOrdering;
use crate::ui::albums_page::AlbumsPage;
use crate::ui::artist_object::ArtistOrdering;
use crate::ui::artists_page::ArtistsPage;
use crate::ui::song_object::SongOrdering;
use crate::ui::songs_page::SongsPage;

#[inline]
pub fn songs_sort_mode(songs_page: SongsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("songs_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state("Default".to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            songs_page.set_sort_mode(match &*variant.get::<String>().unwrap() {
                "Default" => SongOrdering::Default,
                "Rating" => SongOrdering::Rating,
                "Play Count" => SongOrdering::PlayCount,
                "Release Date" => SongOrdering::ReleaseDate,
                "Added" => SongOrdering::Added,
                "Modified" => SongOrdering::Modified,
                _ => unimplemented!(),
            });
        })
        .build()
}
#[inline]
pub fn albums_sort_mode(albums_page: AlbumsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("albums_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state("Default".to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            albums_page.set_sort_mode(match &*variant.get::<String>().unwrap() {
                "Default" => AlbumOrdering::ArtistYearAlbum,
                "Rating" => AlbumOrdering::Rating,
                "Play Count" => AlbumOrdering::PlayCount,
                "Release Date" => AlbumOrdering::ReleaseDate,
                "Added" => AlbumOrdering::Added,
                "Modified" => AlbumOrdering::Modified,
                _ => unimplemented!(),
            });
        })
        .build()
}
#[inline]
pub fn artists_sort_mode(artists_page: ArtistsPage) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("artists_sort_mode")
        .parameter_type(Some(VariantTy::STRING))
        .state("Default".to_variant())
        .activate(move |_, action, variant| {
            let variant = variant.unwrap();
            action.set_state(variant);
            artists_page.set_sort_mode(match &*variant.get::<String>().unwrap() {
                "Default" => ArtistOrdering::Artist,
                "Added" => ArtistOrdering::Added,
                "Modified" => ArtistOrdering::Modified,
                _ => unimplemented!(),
            });
        })
        .build()
}
