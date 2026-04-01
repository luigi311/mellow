use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

mod imp;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::init_channels;
use crate::library::{Library, LibraryConfig, LibraryRequest, library_tx};
use crate::player::{Player, SongQueue};
use crate::shortcuts::Shortcuts;
use crate::ui::{UpdateUI, Window, actions};
use crate::{about, music_dir, util::unescaped_split};

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, adw::Application, gtk::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    #[inline]
    pub fn run() -> glib::ExitCode {
        let app: Self = glib::Object::builder()
            .property("application-id", about::app_id())
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        app.connect_startup(Self::init);
        app.connect_open(Self::open_files);

        app.run()
    }

    #[inline]
    fn init(&self) {
        let (ui_rx, player_rx, library_rx) = init_channels();

        let imp = self.imp();

        imp.player_handle.set(Some(
            thread::Builder::new()
                .name("player".to_owned())
                .spawn(move || Player::init(player_rx).controller().unwrap())
                .unwrap(),
        ));

        let settings = gio::Settings::new(about::app_id());
        let startup_queue = settings.enum_("startup-queue");
        let directories = settings.string("directories");

        imp.library_handle.set(Some(
            thread::Builder::new()
                .name("library".to_owned())
                .spawn(move || {
                    let mut library = Library::init(
                        LibraryConfig::new(match &*directories {
                            // The value ":" means "first launch"
                            ":" => vec![music_dir().clone()],
                            dirs => unescaped_split(dirs, ','),
                        }),
                        library_rx,
                    );
                    library.discover_files();
                    SongQueue::init_queue(&library, startup_queue.into()).unwrap();
                    library.request_handler().unwrap();
                })
                .unwrap(),
        ));

        self.create_window(settings, ui_rx);

        self.setup_actions();
        self.setup_shortcuts();

        self.connect_activate(Self::present_window);
        self.connect_shutdown(Self::shutdown);
    }

    #[inline]
    #[must_use]
    fn window(&self) -> &Window {
        self.imp().window.get().expect(EXP_INIT)
    }

    #[inline]
    fn create_window(
        &self,
        settings: gio::Settings,
        ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>,
    ) {
        let window = Window::new(self, settings);

        glib::spawn_future_local({
            let window = window.clone();
            async move { window.imp().event_handler(ui_rx).await }
        });

        window.set_icon_name(Some(about::app_id()));
        window.set_title(Some(about::app_name()));
        window.present();
        println!("Window presented");

        let _ = self.imp().window.set(window);
    }

    #[inline]
    fn open_files(&self, files: &[gio::File], _: &str) {
        let files = files
            .iter()
            .map(|file| file.path().unwrap().to_str().unwrap().to_owned())
            .collect();
        (library_tx().send(LibraryRequest::QueueFromPaths(files))).expect(EXP_RX);
    }

    #[inline]
    fn present_window(&self) {
        self.window().set_visible(true);
    }

    #[inline]
    fn setup_actions(&self) {
        self.add_action_entries([actions::app::quit(self, self.window())]);
    }

    fn shutdown(&self) {
        let imp = self.imp();
        imp.window.get().unwrap().save_and_uninit().unwrap();
        imp.library_handle.take().unwrap().join().unwrap();
        imp.player_handle.take().unwrap().join().unwrap();
    }
}
