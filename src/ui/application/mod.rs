use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

mod imp;

use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::library::{LIBRARY_TX, Library, LibraryConfig, LibraryRequest};
use crate::player::Player;
use crate::ui::{UpdateUI, Window, actions};
use crate::{MUSIC_DIR, about, unescaped_split};

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
        app.connect_activate(Self::present_window);
        app.connect_shutdown(Self::shutdown);

        app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
        app.set_accels_for_action("window.close", &["<Ctrl>W"]);
        app.set_accels_for_action("win.queue_from_disk", &["<Ctrl>O"]);
        // TODO: Ignore shortcut when the overlay is open
        // app.set_accels_for_action("player.play_pause", &["space"]);

        app.run()
    }

    #[inline]
    fn init(&self) {
        let (player, player_tx, ui_tx, ui_rx) = Player::init();
        thread::Builder::new()
            .name("player".to_owned())
            .spawn(move || player.controller().unwrap())
            .expect(INIT_ERR);

        let settings = gio::Settings::new(about::app_id());
        let startup_queue = settings.enum_("startup-queue");
        let mut library = Library::init(
            LibraryConfig::new(match &*settings.string("directories") {
                // The value ":" means "first launch"
                ":" => vec![MUSIC_DIR.get().unwrap().clone()],
                dirs => unescaped_split(dirs, ','),
            }),
            player_tx,
            ui_tx,
        );
        self.imp().library_handle.set(Some(
            thread::Builder::new()
                .name("library".to_owned())
                .spawn(move || {
                    library.discover_files();
                    library.init_queue(startup_queue).unwrap();
                    library.request_handler().unwrap();
                })
                .expect(INIT_ERR),
        ));

        self.create_window(settings, ui_rx);
        self.setup_actions();
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
        window.set_title(Some(about::app_name()));
        window.set_icon_name(Some(about::app_id()));
        window.present();

        let _ = self.imp().window.set(window.clone());

        glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
    }

    #[inline]
    fn open_files(&self, files: &[gio::File], _: &str) {
        let files = files
            .iter()
            .map(|file| file.path().unwrap().to_str().unwrap().to_owned())
            .collect();
        (LIBRARY_TX.get().expect(EXP_INIT))
            .send(LibraryRequest::QueueFromPaths(files))
            .expect(EXP_RX);
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
    }
}
