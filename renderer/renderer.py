import io
import os
import time
import pystache
from quart import Quart, send_file, request
from contextlib import asynccontextmanager

from playwright.async_api import async_playwright
from playwright.async_api._generated import Playwright as AsyncPlaywright
from playwright._impl._browser import Browser
from playwright._impl._browser_context import BrowserContext
from playwright._impl._page import Page

TEMPLATE_PATH = "./renderer/template"

app = Quart(__name__)


@app.route("/")
async def index():
    return "hello!!"


@app.route("/overall", methods=["POST"])
async def overall():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/overall.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="overall.png",
        mimetype="image/png",
    )


@app.route("/overall_tiers", methods=["POST"])
async def overall_tiers():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/overall_tiers.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="overall_tiers.png",
        mimetype="image/png",
    )


@app.route("/clan_season", methods=["POST"])
async def clan_season():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/clan_season.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="clan_season.png",
        mimetype="image/png",
    )


@app.route("/clan", methods=["POST"])
async def clan():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/clan.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="clan.png",
        mimetype="image/png",
    )


@app.route("/recent", methods=["POST"])
async def recent():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/recent.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="recent.png",
        mimetype="image/png",
    )


@app.route("/leaderboard", methods=["POST"])
async def leaderboard():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/leaderboard.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="leaderboard.png",
        mimetype="image/png",
    )


@app.route("/single_ship", methods=["POST"])
async def single_ship():
    data = await request.get_json()
    html = pystache.Renderer().render_path(f"{TEMPLATE_PATH}/single_ship.html", data)
    return await send_file(
        await renderer.screenshot(html),
        attachment_filename="single_ship.png",
        mimetype="image/png",
    )


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
            with open("./temp/screenshot_output.html", "w", encoding="UTF-8") as f:
                f.write(html)

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
                        "url": "https://ru.wows-numbers.com",
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

    # async def wws_stats_website_update(self, region, user):
    #     _region = NUMBER_REGION[region]
    #     async with self.new_page() as page:
    #         try:
    #             await page.goto(
    #                 f'https://{region}wows-numbers.com/player/{user["uid"]},/'
    #             )
    #             result = await page.query_selector(".loading")
    #             if result == None:
    #                 pass
    #             else:
    #                 await page.wait_for_selector(".loading", state="hidden")
    #         except Exception as e:
    #             # print(e)
    #             pass

    # async def wws_stats_recent_scrape(self, url):
    #     async with self.new_page() as page:
    #         try:
    #             # response = await page.goto(url)
    #             # print(response.headers['status'])
    #             # await page.screenshot({'path': './screen.png', 'fullPage': True})
    #             # await page.waitForSelector('.cf-browser-verification', {'hidden':1})
    #             response = await page.goto(url)
    #             content = await page.content()
    #             if response.ok:
    #                 return content
    #             else:
    #                 return None
    #         except Exception as e:
    #             print(e)
    #             raise utils.IsacError("❌ Update timeout, plz try again")


renderer = None

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=3000, debug=True)