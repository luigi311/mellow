use adw::subclass::prelude::*;
use glib::{clone, variant::StaticVariantType};
use gtk::{gio, glib};

use crate::ui::{Window, library_page::SubpageType};

#[inline]
pub fn open_sheet(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("open_sheet")
        .activate(clone!(
            #[weak(rename_to=ui)]
            window.imp(),
            move |_, _, _| ui.open_sheet(true)
        ))
        .build()
}
#[inline]
pub fn close_sheet(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("close_sheet")
        .activate(clone!(
            #[weak(rename_to=ui)]
            window.imp(),
            move |_, _, _| ui.open_sheet(false)
        ))
        .build()
}
#[inline]
pub fn playing_nav_push(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("playing_nav_push")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(clone!(
            #[weak(rename_to=ui)]
            window.imp(),
            move |_, _, tag| {
                let tag = tag.unwrap().get::<String>().unwrap();
                ui.playing.push_by_tag(&tag);
            }
        ))
        .build()
}
#[inline]
pub fn playing_nav_pop(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("playing_nav_pop")
        .activate(clone!(
            #[weak(rename_to=ui)]
            window.imp(),
            move |_, _, _| {
                ui.playing.pop();
            }
        ))
        .build()
}
// FIX: This should be called when exiting using the back button as well
#[inline]
pub fn library_nav_pop(window: &Window) -> gio::ActionEntry<gio::SimpleActionGroup> {
    gio::ActionEntry::builder("library_nav_pop")
        .activate(clone!(
            #[weak(rename_to=ui)]
            window.imp(),
            move |_, _, _| {
                // FIX: This causes a panic: "RefCell already borrowed"
                // match ui.library_subpages.borrow_mut().pop() {
                //     Some(SubpageType::Song) => {
                //         ui.song_pages.borrow_mut().pop();
                //     }
                //     Some(SubpageType::Album) => {
                //         ui.album_pages.borrow_mut().pop();
                //     }
                //     Some(SubpageType::Artist) => {
                //         ui.artist_pages.borrow_mut().pop();
                //     }
                //     None => return,
                // };
                ui.library.pop();
            }
        ))
        .build()
}
