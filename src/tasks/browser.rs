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

pub async fn launch_renderer() {
    let python = if cfg!(windows) { "python" } else { "python3" };
    // Replace "path/to/executable" with the actual path to your executable
    let _ = tokio::process::Command::new(python)
        .arg("./renderer/renderer.py")
        .spawn()
        .expect("Failed to start subprocess");
}
