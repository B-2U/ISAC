import io
import json
import os
import time
import math
import pystache
from quart import Quart, Response, send_file, request
from contextlib import asynccontextmanager

from playwright.async_api import async_playwright
from playwright.async_api._generated import Playwright as AsyncPlaywright
from playwright._impl._browser import Browser
from playwright._impl._browser_context import BrowserContext
from playwright._impl._page import Page

TEMPLATE_PATH = "./renderer/template"

app = Quart(__name__)


# some numbers like clan_id get formatted too, but it's fine since we don't need them here
def format_big_num_with_commas(value):
    if isinstance(value, dict):
        return {k: format_big_num_with_commas(v) for k, v in value.items()}
    elif isinstance(value, list):
        return [format_big_num_with_commas(item) for item in value]
    # transfer str int to int first
    elif isinstance(value, str):
        if value.isdecimal():
            value = int(value)
        else:
            return value
    if isinstance(value, int):
        if value > 0:
            digits = int(math.log10(value)) + 1
        elif value == 0:
            digits = 1
        else:
            digits = int(math.log10(-value)) + 1
        return "{:,}".format(value) if digits >= 5 else value

    return value


def render_html(template_path: str, data: dict) -> str:
    if os.name != "posix":
        # json for debug
        with open(f"{template_path}.json", "w", encoding="UTF-8") as f:
            json.dump(data, f, indent=2)
    data = format_big_num_with_commas(data)
    return html_renderer.render_path(template_path, data)


async def return_png(bin: io.BytesIO) -> Response:
    return await send_file(
        bin,
        attachment_filename="overall.png",
        mimetype="image/png",
    )


# @app.route("/overall_gif", methods=["POST"])
# async def overall_gif():
#     data = await request.get_json()
#     html = render_html(f"{TEMPLATE_PATH}/overall.hbs", data)
#     video = await renderer.screenshot(html)
#     return await send_file(
#         await renderer.screenshot(html),
#         attachment_filename="single_ship.gif",
#         mimetype="image/gif",
#     )
#     return await return_png(await renderer.screenshot(html))


@app.route("/overall", methods=["POST"])
async def overall():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/overall.hbs", data)
    t = time.time()
    img = await renderer.screenshot(html)
    print(time.time() - t)
    return await return_png(img)


@app.route("/overall_tiers", methods=["POST"])
async def overall_tiers():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/overall_tiers.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.route("/clan_season", methods=["POST"])
async def clan_season():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/clan_season.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.route("/clan", methods=["POST"])
async def clan():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/clan.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.route("/recent", methods=["POST"])
async def recent():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/recent.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.route("/leaderboard", methods=["POST"])
async def leaderboard():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/leaderboard.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.route("/single_ship", methods=["POST"])
async def single_ship():
    data = await request.get_json()
    html = render_html(f"{TEMPLATE_PATH}/single_ship.hbs", data)
    return await return_png(await renderer.screenshot(html))


@app.before_serving
async def startup():
    app.add_background_task(Renderer.launch)


class Renderer:
    def __init__(self):
        self.playwright: AsyncPlaywright = None
        self.browser: Browser = None
        self.context: BrowserContext = None
        self.last_context_created_time: int = 0

    @classmethod
    async def launch(cls, **kwargs):
        self = Renderer()
        if os.name == "posix":
            browser_args = [
                "-allow-file-access-from-files",
                "-disable-web-security",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--disable-gpu",
                "--no-zygote",
                '--js-flags="--max-old-space-size=300"',
            ]  # for linux
        else:
            browser_args = [
                "-allow-file-access-from-files",
                "-disable-web-security",
                "--no-sandbox",
            ]
        self.playwright: AsyncPlaywright = await async_playwright().start()
        self.browser = await self.playwright.chromium.launch(
            args=browser_args, headless=True
        )
        global renderer
        renderer = self
        return self

    async def screenshot(self, html) -> io.BytesIO:
        if os.name != "posix":
            # html for debug
            try:
                with open("./temp/screenshot_output.html", "w", encoding="UTF-8") as f:
                    f.write(html)
            except:
                pass

        async with self.new_page() as page:
            await page.set_content(html)
            return io.BytesIO(await page.locator(".main").screenshot())

    async def get_context(self) -> BrowserContext:
        now = time.time()
        if now - self.last_context_created_time > 3600:
            if self.context:
                await self.context.close()
            self.last_context_created_time = now
            self.context = await self.browser.new_context()
            await self.context.add_cookies(
                [
                    {
                        "name": "apiConsent",
                        "value": "1",
                        "url": "https://asia.wows-numbers.com",
                    },
                    {
                        "name": "apiConsent",
                        "value": "1",
                        "url": "https://na.wows-numbers.com",
                    },
                    {
                        "name": "apiConsent",
                        "value": "1",
                        "url": "https://wows-numbers.com",
                    },
                ]
            )
        else:
            # use current context
            pass
        return self.context

    @asynccontextmanager
    async def new_page(self, **kwargs) -> Page:
        context = await self.get_context()
        page = await context.new_page(**kwargs)
        await page.goto(f"file://{os.getcwd()}")
        try:
            yield page
        finally:
            await page.close()


renderer = None
html_renderer = pystache.Renderer()

if __name__ == "__main__":
    app.run(port=3000, debug=True)

# TODO 為什麼htop下會有野生的renderer.py?
