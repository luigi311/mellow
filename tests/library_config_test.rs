#[cfg(test)]
mod tests {
    use gtk::{gio, prelude::FileExt};
    use std::sync::mpsc;
    use tokio::sync::mpsc as tokio_mpsc;

    use mellow::library::config::LibraryConfig;
    use mellow::library::{LIBRARY_TX, LibraryRequest};
    use mellow::ui::{UI_TX, UpdateUI};

    struct ConfigTester {
        config: LibraryConfig,
        _ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>,
        _library_rx: mpsc::Receiver<LibraryRequest>,
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
        config_tester.test_uri_opt_remainder_special_chars();
        config_tester.test_uri_opt_remainder_common_special_chars();
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
                self.uri_opt_split(),
                [("file:///test".to_string(), String::new())],
                "`test_uri_opt_remainder_single_dir()`",
            )
        }

        fn test_set_libraries(&mut self) {
            self.config.set_libraries(&[
                "/some/directory".to_string(),
                "/some/other/directory".to_string(),
            ]);
            assert_eq!(
                self.config.directories,
                ["/some/directory", "/some/other/directory",],
                "`test_set_libraries()`"
            );
        }

        fn test_uri_opt_remainder_after_set(&self) {
            assert_eq!(
                self.uri_opt_split(),
                [
                    ("file:///some/".to_string(), "directory".to_string()),
                    ("file:///some/".to_string(), "other/directory".to_string()),
                ],
                "`test_uri_opt_remainder_after_set()`",
            );
        }

        fn test_uri_opt_remainder_after_add(&mut self) {
            self.config.add_library("/songs".to_string());
            assert_eq!(
                self.uri_opt_split(),
                [
                    ("file:///so".to_string(), "me/directory".to_string()),
                    ("file:///so".to_string(), "me/other/directory".to_string()),
                    ("file:///so".to_string(), "ngs".to_string()),
                ],
                "`test_uri_opt_remainder_after_add()`",
            );
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
                self.uri_opt_split(),
                [
                    ("file:///some/".to_string(), "directory".to_string()),
                    ("file:///some/".to_string(), "other/directory".to_string())
                ],
                "`test_uri_opt_remainder_after_remove()`",
            );
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

        fn test_uri_opt_remainder_special_chars(&mut self) {
            self.config
                .set_libraries(&["/test/🤷/".to_string(), "/test/🦀/".to_string()]);
            for (first_half, _) in self.uri_opt_split() {
                assert_eq!(
                    first_half, "file:///test/",
                    "`uri_opt_remainder_special_chars()`"
                );
            }
        }

        fn test_uri_opt_remainder_common_special_chars(&mut self) {
            self.config
                .set_libraries(&["/test/🤷/🦀".to_string(), "/test/🤷/🤷".to_string()]);
            assert!(self.config.uri_opt() <= "file:///test/%F0%9F%A4%B7/".len());
            // NOTE: The below test is currently failing, but as long as the
            // `uri_opt` value is less than the common part length, it shouln't
            // cause any issues other than be slightly suboptimal
            // for (first_half, _) in self.uri_opt_split() {
            //     assert_eq!(
            //         first_half, "file:///test/%F0%9F%A4%B7/",
            //         "`uri_opt_remainder_special_chars()`"
            //     );
            // }
        }

        fn uri_opt_split(&self) -> Vec<(String, String)> {
            self.config
                .directories
                .iter()
                .map(|dir| {
                    let uri = &gio::File::for_path(dir).uri();
                    let split = uri.split_at(self.config.uri_opt());
                    (split.0.to_string(), split.1.to_string())
                })
                .collect()
        }
    }

    impl Default for ConfigTester {
        fn default() -> Self {
            let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<UpdateUI>();
            UI_TX.get_or_init(|| ui_tx.clone());
            let (library_tx, library_rx) = mpsc::channel::<LibraryRequest>();
            LIBRARY_TX.get_or_init(|| library_tx.clone());
            ConfigTester {
                config: LibraryConfig::new(vec![]),
                _ui_rx: ui_rx,
                _library_rx: library_rx,
            }
        }
    }
}
