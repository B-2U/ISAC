import os
import json
import asyncio
import aiohttp

# from wowspy import WowsAsync, Wows
from datetime import datetime, timezone, timedelta
import time
import traceback
from multiprocessing import Process
from apscheduler.schedulers.blocking import BlockingScheduler

sched = BlockingScheduler(timezone="GMT", job_defaults={"misfire_grace_time": 30 * 60})

api_key = "40651eccb00abf54ba1135af751ced1e"
RECENT_LAST_REQUEST_LIMIT = 14
MAX_DAY = 121


def domain(region):
    official_domain = {"asia": "asia", "na": "com", "eu": "eu", "ru": "ru"}
    return official_domain[region]


###
# return empty dict if player is hidden
###
async def statistics_of_players_ships(
    session,
    region: str,
    account_id: int,
) -> dict:
    modes = ["pvp_solo", "pvp_div2", "pvp_div3", "rank_solo", "pvp"]
    sub_modes = ["pvp_solo", "pvp_div2", "pvp_div3", "rank_solo"]

    async def fetch(session: aiohttp.ClientSession, url: str) -> dict:
        async with session.get(url) as res:
            return await res.json()

    jobs = [
        fetch(
            session,
            f"https://vortex.worldofwarships.{domain(region)}/api/accounts/{account_id}/ships/{mode}",
        )
        for mode in modes
    ]
    responses = await asyncio.gather(*jobs)
    main_res = responses.pop(-1)
    if main_res["data"].get("hidden_profile") == True:
        return main_res  # hidden, dont need to merge and clean, let parent function handle it

    # merging 5 responses
    for sub_mode_name, res in zip(sub_modes, responses):
        for ship_id in main_res["data"][str(account_id)]["statistics"]:
            main_res["data"][str(account_id)]["statistics"][str(ship_id)][
                sub_mode_name
            ] = res["data"][str(account_id)]["statistics"][str(ship_id)][sub_mode_name]

    # clean up empty
    new_data = {}
    for ship_id, stats in main_res["data"][str(account_id)]["statistics"].items():
        new_stats = {}
        for mode, mode_stats in stats.items():
            if mode_stats.get("battles_count", 0) != 0:
                # reserve the keys we wanted
                trim_mode_stats = {
                    "battles_count": mode_stats["battles_count"],
                    "wins": mode_stats["wins"],
                    "damage_dealt": mode_stats["damage_dealt"],
                    "frags": mode_stats["frags"],
                    "planes_killed": mode_stats["planes_killed"],
                    "original_exp": mode_stats["original_exp"],
                    "art_agro": mode_stats["art_agro"],
                    "scouting_damage": mode_stats["scouting_damage"],
                    "shots_by_main": mode_stats["shots_by_main"],
                    "hits_by_main": mode_stats["hits_by_main"],
                }
                new_stats[mode] = trim_mode_stats
        if new_stats:
            new_data[ship_id] = new_stats

    main_res["data"][str(account_id)]["statistics"] = new_data

    return main_res


async def update(region: str, now: int):
    async with aiohttp.ClientSession() as session:
        # my_api = WowsAsync(api_key, session)
        # fields = "ship_id, rank_solo, rank_solo.battles, rank_solo.frags, rank_solo.planes_killed, rank_solo.wins, rank_solo.damage_dealt, pvp.battles, pvp.frags, pvp.planes_killed, pvp.wins, pvp.damage_dealt, pvp_div2, pvp_div2.battles, pvp_div2.frags, pvp_div2.planes_killed, pvp_div2.wins, pvp_div2.damage_dealt, pvp_div3, pvp_div3.battles, pvp_div3.frags, pvp_div3.planes_killed, pvp_div3.wins, pvp_div3.damage_dealt,, pvp_solo, pvp_solo.battles, pvp_solo.frags, pvp_solo.planes_killed, pvp_solo.wins, pvp_solo.damage_dealt"

        files = os.listdir(f"./players/{region}/")
        # with open("../user_data/pfp.json", "r") as f:
        #     pfp_js = json.load(f)
        if files == None:
            return
        for file in files:
            try:
                uid = int(file.split(".")[0])  # 12345678 .json
                with open(f"./players/{region}/{file}", encoding="UTF-8") as f:
                    player_js = json.load(f)

                # active check
                if player_js["last_request"] == "Premium":
                    active = True
                else:
                    if isinstance(player_js["last_request"], dict):
                        # new format
                        last_request_time: int = player_js["last_request"]["Normal"]
                    else:
                        # old format
                        last_request_time: int = player_js["last_request"]

                    active = (
                        True
                        if now - last_request_time < RECENT_LAST_REQUEST_LIMIT * 86400
                        else False
                    )

                if active:  # premium or active
                    res = await statistics_of_players_ships(session, region, uid)
                    if res["data"].get("hidden_profile") == True:
                        continue  # hidden
                    current_ships = res["data"][str(uid)]["statistics"]
                else:  # not active anymore, pass it
                    current_ships = {}

                wait_for_pop = []
                # only for the loop belowed
                for key, value in player_js["data"].items():
                    limit = MAX_DAY * 86400

                    if now - int(key) >= limit:  # over ttl, remove it
                        wait_for_pop.append(key)
                    elif value == current_ships:  # same as today, remove it
                        wait_for_pop.append(key)

                for key in set(wait_for_pop):
                    player_js["data"].pop(key)

                # put in today's new snapshot
                if current_ships:
                    player_js["data"][str(now)] = current_ships
                    player_js["last_update_at"] = now

                if player_js["data"]:
                    with open(f"./players/{region}/{file}", "w") as f:
                        json.dump(player_js, f)
                else:  # delete players' with no data left
                    os.remove(f"./players/{region}/{file}")

            except KeyError:
                pass

            except Exception as e:
                print(f"{region} {file}")
                print(traceback.format_exc())


def tw_timestamp():
    tpe = datetime.now(timezone(timedelta(hours=+8)))
    return tpe.strftime("[%m/%d %H:%M]")


def create_process(region: str, now: int):
    p = Process(target=async_run_update, args=(region, now))
    p.start()
    p.join()


def async_run_update(region: str, now: int):
    asyncio.run(update(region, now))


@sched.scheduled_job("cron", hour="5")  # 1PM UTC+8
def EU_update():
    start = int(time.time())
    create_process("eu", start)
    print(f"{tw_timestamp()} EU updated, time took: {int(time.time())- start}s")


@sched.scheduled_job("cron", hour="10")  # 6PM UTC+8
def NA_update():
    start = int(time.time())
    create_process("na", start)
    print(f"{tw_timestamp()} NA updated, time took: {int(time.time())- start}s")


@sched.scheduled_job("cron", hour="21")  # 5AM UTC+8
def ASIA_update():
    start = int(time.time())
    create_process("asia", start)
    print(f"{tw_timestamp()} ASIA updated, time took: {int(time.time())- start}s")


async def test():
    async with aiohttp.ClientSession() as session:
        res = await statistics_of_players_ships(session, "asia", 2025455227)
        print(res)


if __name__ == "__main__":
    asyncio.run(test())
    # asyncio.run(ASIA_update())
    sched.start()
