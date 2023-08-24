import os
import json


def main():
    for region in ["asia", "eu", "na"]:
        files = os.listdir(f"./players/{region}/")
        # with open("../user_data/pfp.json", "r") as f:
        #     pfp_js = json.load(f)
        if files == None:
            return
        for file in files:
            uid = int(file.split(".")[0])  # 12345678 .json
            clean(f"./players/{region}/{file}")


def clean(path):
    with open(path, encoding="UTF-8") as f:
        player_js = json.load(f)

    if isinstance(player_js["last_request"], int):
        player_js["last_request"] = {"Normal": player_js["last_request"]}
    elif player_js["last_request"] == "prime":
        player_js["last_request"] = "Premium"

    # clean up no stats modes
    new_data = {}
    for date, snap in player_js["data"].items():
        new_snap = {}
        for ship_id, modes in snap.items():
            new_modes = {}
            for mode_name, mode_stats in modes.items():
                if mode_stats.get("battles") != None:
                    mode_stats["battles_count"] = mode_stats.pop("battles")
                if mode_stats["battles_count"] == 0:
                    continue
                new_modes[mode_name] = mode_stats
            if new_modes != {}:
                new_snap[ship_id] = new_modes
        if new_snap != {}:
            new_data[date] = new_snap

    current_time, current_ships = new_data.popitem()

    print(len(new_data))
    new_data = keep_last_of_each_value(new_data)
    print(len(new_data))

    wait_for_pop = []
    for key, value in new_data.items():
        if value == current_ships:  # same as today, remove it
            wait_for_pop.append(key)

    for key in set(wait_for_pop):
        new_data.pop(key)

    # put in today's new snapshot
    if current_ships:
        new_data[current_time] = current_ships

    player_js["data"] = new_data

    with open(path, "w") as f:
        json.dump(player_js, f)


def keep_last_of_each_value(d) -> dict:
    seen_values = []
    new_dict = {}

    for key, value in reversed(d.items()):
        if value not in seen_values:
            seen_values.append(value)
            new_dict[key] = value

    new_dict = dict(reversed(new_dict.items()))

    return new_dict


# main()
