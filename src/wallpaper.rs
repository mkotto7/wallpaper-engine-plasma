use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use zbus::zvariant::Value;

#[derive(Deserialize, Debug)]
struct Screen {
    id: u16,
}

pub async fn set_wallpaper(path: &Path, screen: u32, fill_mode: u8) -> anyhow::Result<()> {
    // ref: https://github.com/KDE/plasma-workspace/blob/master/wallpapers/image/plasma-apply-wallpaperimage.cpp
    let conn = zbus::Connection::session().await?;

    // image parameters: /usr/share/plasma/wallpapers/org.kde.image/contents
    let mut params: HashMap<String, Value> = HashMap::new();
    params.insert("Image".to_string(), Value::from(path.display().to_string()));
    params.insert("FillMode".to_string(), Value::from(fill_mode.to_string()));

    conn.call_method(
        Some("org.kde.plasmashell"),
        "/PlasmaShell",
        Some("org.kde.PlasmaShell"),
        "setWallpaper",
        &("org.kde.image", params, screen),
    )
    .await?;

    println!("Applied wallpaper: {}", path.display());
    println!(
        "Screen: {}\nFill mode: {}\n",
        screen,
        fill_mode.to_string()
    );
    Ok(())
}

pub async fn get_screens() -> anyhow::Result<String> {
    let script = "function ds() {
        var info = [];
        var ds = desktops();
        for (var i = 0; i < ds.length; i++) {
            var d = ds[i];
            info.push({
                id: i
            });
        }
        return JSON.stringify(info);
    }
    print(ds())"
        .to_string();

    let conn = zbus::Connection::session().await?;

    let (screens,): (String,) = conn
        .call_method(
            Some("org.kde.plasmashell"),
            "/PlasmaShell",
            Some("org.kde.PlasmaShell"),
            "evaluateScript",
            &(&script),
        )
        .await?
        .body()
        .deserialize()?;

    let parsed: Vec<Screen> = serde_json::from_str(&screens).expect("Could not get screens");
    let mut output = String::new();
    for screen in parsed {
        output.push_str(&format!("id: {}\n", screen.id))
    }
    Ok(output)
}
