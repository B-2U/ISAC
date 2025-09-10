// pub async fn launch_webdriver() {
//     let path = if cfg!(windows) {
//         "./webdriver/chromedriver.exe"
//     } else {
//         "./webdriver/chromedriver"
//     };
//     // Replace "path/to/executable" with the actual path to your executable
//     let _ = tokio::process::Command::new(path)
//         .arg("--port=4444")
//         .arg("--headless=new")
//         .spawn()
//         .expect("Failed to start subprocess");
// }

use std::env;

pub async fn launch_renderer() -> tokio::process::Child {
    let python = env::var("PYTHON_PATH").unwrap_or("python".to_string());
    // Replace "path/to/executable" with the actual path to your executable
    tokio::process::Command::new(python)
        .arg("./renderer/renderer.py")
        .spawn()
        .unwrap_or_else(|err| panic!("Failed to start subprocess: {}", err))
}
