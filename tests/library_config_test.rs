#[cfg(test)]
mod tests {
    use gtk::{gio, prelude::FileExt};
    use tokio::sync::mpsc as tokio_mpsc;

    use mellow::library::config::LibraryConfig;
    use mellow::ui::{UI_TX, UpdateUI};

    struct ConfigTester {
        config: LibraryConfig,
        _ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>,
    }

    #[test]
    fn library_config_correctness() {
        let mut config_tester = ConfigTester::default();
        config_tester.test_empty_by_default();
        config_tester.test_add_library();
        config_tester.test_uri_opt_remainder_single_dir();
        config_tester.test_set_libraries();
        config_tester.test_uri_opt_remainder_after_set();
        config_tester.test_uri_opt_remainder_after_add();
        config_tester.test_remove_library();
        config_tester.test_uri_opt_remainder_after_remove();
        config_tester.test_sort_alphabetically();
        config_tester.test_reject_duplicates();
        config_tester.test_reject_empty();
    }

    impl ConfigTester {
        fn test_empty_by_default(&self) {
            assert!(&self.config.directories.is_empty());
        }

        fn test_add_library(&mut self) {
            self.config.add_library("/test".to_string());
            assert_eq!(self.config.directories, ["/test"], "`test_add_library()`");
        }

        fn test_uri_opt_remainder_single_dir(&mut self) {
            assert_eq!(
                self.config
                    .directories
                    .iter()
                    .map(|dir| gio::File::for_path(dir).uri()[self.config.uri_opt()..].to_string())
                    .collect::<Vec<_>>(),
                [""],
                "`test_uri_opt_single()`",
            )
        }

        fn test_set_libraries(&mut self) {
            // Test setting directories
            self.config.set_libraries(
                &[
                    "/some/directory".to_string(),
                    "/some/other/directory".to_string(),
                ],
                &UI_TX.get().unwrap(),
            );
            assert_eq!(
                self.config.directories,
                ["/some/directory", "/some/other/directory",],
                "`test_set_libraries()`"
            );
        }

        fn test_uri_opt_remainder_after_set(&self) {
            assert_eq!(
                self.config
                    .directories
                    .iter()
                    .map(|dir| gio::File::for_path(dir).uri()[self.config.uri_opt()..].to_string())
                    .collect::<Vec<_>>(),
                ["directory", "other/directory"],
                "`test_uri_opt_after_set()`",
            )
        }

        fn test_uri_opt_remainder_after_add(&mut self) {
            self.config.add_library("/songs".to_string());
            assert_eq!(
                self.config
                    .directories
                    .iter()
                    .map(|dir| gio::File::for_path(dir).uri()[self.config.uri_opt()..].to_string())
                    .collect::<Vec<_>>(),
                ["me/directory", "me/other/directory", "ngs"],
                "`test_uri_opt_after_add()`",
            )
        }

        fn test_remove_library(&mut self) {
            self.config.remove_library(2);
            assert_eq!(
                self.config.directories,
                ["/some/directory", "/some/other/directory"],
                "`test_remove_library()`"
            );
        }

        fn test_uri_opt_remainder_after_remove(&mut self) {
            assert_eq!(
                self.config
                    .directories
                    .iter()
                    .map(|dir| gio::File::for_path(dir).uri()[self.config.uri_opt()..].to_string())
                    .collect::<Vec<_>>(),
                ["directory", "other/directory"],
                "`test_uri_opt_after_remove()`",
            )
        }

        fn test_sort_alphabetically(&mut self) {
            self.config.add_library("/audio".to_string());
            assert_eq!(
                self.config.directories,
                ["/audio", "/some/directory", "/some/other/directory",],
                "`test_sort_alphabetically()`"
            );
        }

        fn test_reject_duplicates(&mut self) {
            self.config.add_library("/some/directory".to_string());
            assert_eq!(
                self.config.directories,
                ["/audio", "/some/directory", "/some/other/directory",],
                "`test_reject_duplicates()`"
            );
        }

        fn test_reject_empty(&mut self) {
            self.config.add_library("".to_string());
            assert_eq!(
                self.config.directories,
                ["/audio", "/some/directory", "/some/other/directory",],
                "`test_reject_empty()`"
            );
        }
    }

    impl Default for ConfigTester {
        fn default() -> Self {
            let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<UpdateUI>();
            UI_TX.get_or_init(|| ui_tx.clone());
            ConfigTester {
                config: LibraryConfig::default(),
                _ui_rx: ui_rx,
            }
        }
    }
}
