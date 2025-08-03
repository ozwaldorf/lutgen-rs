use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpg", "jpeg", "png", "pnm", "qoi",
    "tga", "tiff", "webp",
];

pub struct FileDialog {
    #[cfg(not(target_arch = "wasm32"))]
    inner: egui_file_dialog::FileDialog,
    #[cfg(not(target_arch = "wasm32"))]
    save: bool,

    #[cfg(target_arch = "wasm32")]
    inner: rfd::AsyncFileDialog,
    #[cfg(target_arch = "wasm32")]
    fut: Option<poll_promise::Promise<Option<(PathBuf, Vec<u8>)>>>,

    ctx: egui::Context,
}

// Wrapper around egui_file_dialog for native support
#[cfg(not(target_arch = "wasm32"))]
impl FileDialog {
    pub fn save(ctx: egui::Context) -> Self {
        let mut inner = egui_file_dialog::FileDialog::new().title("Save Image As");
        for &ext in IMAGE_EXTENSIONS {
            inner = inner.add_save_extension(ext, ext);
        }
        inner = inner.default_save_extension("png");
        Self {
            inner,
            ctx,
            save: true,
        }
    }

    pub fn pick(ctx: egui::Context) -> Self {
        let inner = egui_file_dialog::FileDialog::new()
            .add_file_filter_extensions("Images", IMAGE_EXTENSIONS.to_vec())
            .default_file_filter("Images")
            .title("Open Image");
        Self {
            inner,
            ctx,
            save: false,
        }
    }

    pub fn trigger(&mut self, hint: Option<PathBuf>) {
        if self.save {
            if let Some(path) = hint {
                let config = self.inner.config_mut();
                if let Some(parent) = path.parent() {
                    config.initial_directory = parent.to_path_buf();
                }
                if let Some(file) = path.file_name() {
                    config.default_file_name = file.display().to_string();
                }
            }
            self.inner.save_file();
        } else {
            if let Some(path) = hint {
                let config = self.inner.config_mut();
                if let Some(parent) = path.parent() {
                    config.initial_directory = parent.to_path_buf();
                }
            }
            self.inner.pick_file();
        }
    }

    pub fn poll(&mut self) -> Option<PathBuf> {
        self.inner.update(&self.ctx);
        self.inner.take_picked()
    }
}

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn pick(ctx: egui::Context) -> Self {
        Self {
            inner: rfd::AsyncFileDialog::new(),
            fut: None,
            ctx,
        }
    }

    pub fn trigger(&mut self, _hint: Option<PathBuf>) {
        if self.fut.is_none() {
            let inner = self.inner.clone();
            let ctx = self.ctx.clone();
            self.fut = Some(poll_promise::Promise::spawn_local(async move {
                let Some(handle) = inner.pick_file().await else {
                    return None;
                };
                let bytes = handle.read().await;
                ctx.request_repaint();
                Some((handle.file_name().into(), bytes))
            }));
        }
    }

    pub fn poll(&mut self) -> Option<(PathBuf, Vec<u8>)> {
        let fut = self.fut.take()?;
        match fut.try_take() {
            Ok(v) => {
                self.inner = rfd::AsyncFileDialog::new();
                v
            },
            Err(fut) => {
                self.fut = Some(fut);
                None
            },
        }
    }
}
