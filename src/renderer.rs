use bytes::Bytes;
use handlebars::Handlebars;
use headless_chrome::{protocol::cdp::Page::CaptureScreenshotFormatOption, Browser, LaunchOptions};
use serde::Serialize;
use std::fs;

use crate::utils::{IsacError, IsacInfo};

pub struct Renderer {
    browser: Browser,
    reg: Handlebars<'static>,
}

impl Renderer {
    pub fn launch() -> Self {
        const TEMPLATE_PATH: &str = "./renderer/template/";
        // let args = [
        //     "-allow-file-access-from-files",
        //     "-disable-web-security",
        //     "--no-sandbox",
        // ]
        // .into_iter()
        // .map(OsStr::new)
        // .collect();

        let browser = Browser::new(
            LaunchOptions::default_builder()
                .devtools(false)
                // .args(args)
                .build()
                .unwrap(),
        )
        .unwrap();
        let mut reg = Handlebars::new();

        let templates = fs::read_dir(TEMPLATE_PATH)
            .unwrap()
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_file() && path.extension().unwrap_or_default() == "hbs" {
                        Some(path)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();
        for path in templates {
            reg.register_template_file(path.file_stem().unwrap().to_str().unwrap(), &path)
                .unwrap();
        }
        Self { browser, reg }
    }

    pub async fn render(&self, template: &str, data: impl Serialize) -> Result<Bytes, IsacError> {
        let js_script = format!(
            "document.write(`{}`);",
            self.reg
                .render(template, &data)
                .unwrap()
                .replace("`", "\\`") // Escape backticks in the HTML content
        );
        let tab = self.browser.new_tab().unwrap();
        tab.evaluate(&js_script, false).unwrap();
        let img = tab
            .find_element(".main")
            .and_then(|element| element.capture_screenshot(CaptureScreenshotFormatOption::Png))
            .map_err(|_| IsacInfo::GeneralError {
                msg: "screenshot failed".to_string(),
            })?;
        let _ = tab.close_target();
        Ok(Bytes::from(img))
    }
}

#[test]
fn lauch_renderer() {
    Renderer::launch();
}
