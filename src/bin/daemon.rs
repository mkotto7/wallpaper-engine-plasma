use std::fs::read_dir;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use wallpaper_engine_plasma::validation::is_valid_image;
use wallpaper_engine_plasma::wallpaper::get_screens;
use wallpaper_engine_plasma::{image_generation::generate_image, wallpaper::set_wallpaper};
use zbus::{connection, interface};

struct EngineState {
    directory: Option<PathBuf>,
    loop_enabled: bool,
    period: Duration,
    screen: u32,
    fill_mode: u8,
}

struct Engine {
    state: Arc<Mutex<EngineState>>,
}

#[interface(name = "org.wallpaper.PlasmaEngine")]
impl Engine {
    async fn generate_from_prompt_and_apply(
        &self,
        prompt: String,
        use_cpu: bool,
        screen: u32,
        fill_mode: u8,
    ) -> String {
        let path_result = generate_image(prompt, use_cpu);

        match path_result {
            Ok(path) => match set_wallpaper(&path, screen, fill_mode).await {
                Ok(_) => path.display().to_string(),
                Err(e) => format!("Error setting wallpaper: {}", e),
            },
            Err(e) => format!("Error generating image: {}", e),
        }
    }

    async fn set_image(&self, path: String, screen: u32, fill_mode: u8) -> bool {
        match set_wallpaper(std::path::Path::new(&path), screen, fill_mode).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn get_screens(&self) -> String {
        get_screens()
            .await
            .unwrap_or_else(|e| format!("Error getting screens: {}", e))
    }

    async fn start_directory_loop(
        &self,
        dir: String,
        screen: u32,
        fill_mode: u8,
        period_secs: u64,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        state.directory = Some(PathBuf::from(&dir));
        state.screen = screen;
        state.fill_mode = fill_mode;
        state.period = Duration::from_secs(period_secs);
        state.loop_enabled = true;
        println!("Starting directory loop with:");
        println!("Directory: {}", &dir);
        println!("Screen: {}", screen);
        println!("Fill mode: {}", fill_mode);
        println!("Period: {}s\n", period_secs);
        true
    }

    async fn stop_loop(&self) -> bool {
        println!("Signal received, stopping loop...");
        let mut state = self.state.lock().unwrap();
        state.loop_enabled = false;
        true
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let shared_state = Arc::new(Mutex::new(EngineState {
        directory: None,
        loop_enabled: false,
        period: Duration::from_secs(3600),
        screen: 0,
        fill_mode: 2,
    }));

    let loop_state = Arc::clone(&shared_state);
    tokio::spawn(async move {
        let mut index = 0;
        loop {
            let settings = {
                let s = loop_state.lock().unwrap();
                if !s.loop_enabled || s.directory.is_none() {
                    None
                } else {
                    Some((
                        s.directory.clone().unwrap(),
                        s.period,
                        s.screen,
                        s.fill_mode,
                    ))
                }
            };

            if let Some((dir, period, screen, fill_mode)) = settings {
                let files: Vec<PathBuf> = read_dir(&dir)
                    .map(|rd| {
                        rd.filter_map(|f| f.ok())
                            .map(|f| f.path())
                            .filter(|f| is_valid_image(f))
                            .collect()
                    })
                    .unwrap_or_default();

                if !files.is_empty() {
                    if index >= files.len() {
                        index = 0;
                    }
                    let path = &files[index];

                    let _ = set_wallpaper(path, screen, fill_mode).await;

                    index += 1;

                    sleep(period).await;
                } else {
                    println!("No valid images found in {:?}", dir);
                    sleep(Duration::from_secs(10)).await;
                }
            } else {
                sleep(Duration::from_secs(5)).await;
                index = 0;
            }
        }
    });

    let engine = Engine {
        state: Arc::clone(&shared_state),
    };

    let _conn = connection::Builder::session()?
        .name("org.wallpaper.PlasmaEngine")?
        .serve_at("/org/wallpaper/PlasmaEngine", engine)?
        .build()
        .await?;

    println!("Listening for calls...");
    std::future::pending::<()>().await;
    Ok(())
}
