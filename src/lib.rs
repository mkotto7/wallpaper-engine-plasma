pub mod image_generation;
pub mod validation;
pub mod wallpaper;

use zbus::proxy;

#[derive(Debug, Clone, Copy, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum FillMode {
    Stretch = 0,
    Fit = 1,
    Crop = 2,
    Tile = 3,
    TileVertical = 4,
    TileHorizontal = 5,
    Pad = 6,
}

impl FillMode {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Stretch => 0,
            Self::Fit => 1,
            Self::Crop => 2,
            Self::Tile => 3,
            Self::TileVertical => 4,
            Self::TileHorizontal => 5,
            Self::Pad => 6,
        }
    }
}

#[proxy(
    interface = "org.wallpaper.PlasmaEngine",
    default_service = "org.wallpaper.PlasmaEngine",
    default_path = "/org/wallpaper/PlasmaEngine"
)]
pub trait WallpaperEngine {
    fn generate_from_prompt_and_apply(
        &self,
        prompt: &str,
        use_cpu: bool,
        screen: u32,
        fill_mode: u8,
    ) -> zbus::Result<String>;
    fn set_image(&self, path: &str, screen: u32, fill_mode: u8) -> zbus::Result<bool>;
    fn get_screens(&self) -> zbus::Result<String>;
    fn start_directory_loop(
        &self,
        dir: &str,
        screen: u32,
        fill_mode: u8,
        period_secs: u64,
    ) -> zbus::Result<bool>;
    fn stop_loop(&self) -> zbus::Result<()>;
}
