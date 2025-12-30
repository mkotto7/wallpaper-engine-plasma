use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use dbus::arg::Variant;
use dbus::blocking::Connection;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Screen {
    id: u16,
}

pub fn set_wallpaper(path: &Path, screen: u32, fill_mode: u8) {
    // ref: https://github.com/KDE/plasma-workspace/blob/master/wallpapers/image/plasma-apply-wallpaperimage.cpp
    // dbus ref: https://docs.rs/crate/dbus/latest/source/examples/client.rs

    let conn = Connection::new_session().expect("Failed to connect to daemon");
    let proxy = conn.with_proxy(
        "org.kde.plasmashell",
        "/PlasmaShell",
        Duration::from_millis(5000),
    );

    // image parameters: /usr/share/plasma/wallpapers/org.kde.image/contents
    let screen: u32 = screen;
    let fill_mode = fill_mode.to_string();
    let mut params: HashMap<String, Variant<String>> = HashMap::new();
    params.insert("Image".to_string(), Variant(path.display().to_string()));
    params.insert("FillMode".to_string(), Variant(fill_mode));

    let (): () = proxy
        .method_call(
            "org.kde.PlasmaShell",
            "setWallpaper",
            ("org.kde.image", params, screen),
        )
        .expect("Daemon call failed");

    println!("Applied wallpaper: {}", path.display());
}

pub fn get_screens() -> String {
    let conn = Connection::new_session().expect("Failed to create a connection");
    let proxy = conn.with_proxy(
        "org.kde.plasmashell",
        "/PlasmaShell",
        Duration::from_millis(5000),
    );

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

    let (screens,): (String,) = proxy
        .method_call("org.kde.PlasmaShell", "evaluateScript", (&script,))
        .expect("Daemon call failed");

    let parsed: Vec<Screen> = serde_json::from_str(&screens).expect("Could not get screens");
    let mut output = String::new();
    for screen in parsed {
        output.push_str(&format!("id: {}\n", screen.id))
    }
    output
}
