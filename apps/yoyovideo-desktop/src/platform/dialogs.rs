use std::path::PathBuf;

pub trait DialogService {
    fn pick_file(&self) -> Option<PathBuf>;
    fn pick_folder(&self) -> Option<PathBuf>;
    fn prompt_url(&self) -> Option<String>;
}

#[derive(Default)]
pub struct RfdDialogService;

impl DialogService for RfdDialogService {
    fn pick_file(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }

    fn pick_folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    fn prompt_url(&self) -> Option<String> {
        None
    }
}
