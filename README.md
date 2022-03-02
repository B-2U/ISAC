![](https://cdn.discordapp.com/attachments/865923985891524638/936154921991041024/unknown.png)

**English** | [中文](https://github.com/B-2U/ISAC---wws-stats-bot/blob/main/README_zh.md) | [日本語](https://github.com/B-2U/ISAC---wws-stats-bot/blob/main/README_ja.md)

## A World of Warships stats discord bot

Want to add it to your server? Click [here](https://discord.com/api/oauth2/authorize?client_id=932457496642224178&permissions=0&scope=bot)
Or join our Discord server? [Here](https://discord.gg/z6sV6kEZGV)

---

## Commands list

### **❗ Arguments in `[]` are optional**

#### In the following tables, `<player>` can be:  
  | valid values | example |  
  |-|-|
  | `[region] <ign>` | `.wws asia B2U` or `.wws B2U` |  
  | `<@mention>` | `.wws @B2U#0900` |  
  | `<discordUserID>` | `.wws 930855839961591849` |  
  | `me` | `.wws me` |


| Command name     | Usage                                        | Example                                                     | Description |
|------------------|----------------------------------------------|-------------------------------------------------------------|-------------|
| `.link`          | `.link <region> <ign>` | `.link asia B2U`      | Link your wows account<br>Afterwards you can use `me` as a shortcut to `[region] <ign>`  |
| `.wws`           | `.wws <player>`                        | `.wws asia B2U`<br>`.wws B2U`<br>`.wws me`                  | Show player's stats overview |
| `.wws ship`      | `.wws <player> <shipName> [battleType]`                 | `.wws asia B2U Ise`<br>`.wws B2U Yamato`<br>`.wws me Slava rank` | Show player's stats of a particular ship<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`          |
| `.recent`        | `.recent <player> [battleType] [days]` | `.recent asia B2U`<br>`.recent B2U 7`<br>`.recent me rank 21`     | Show player's recent stats<br>`[days]`: `1`~`21` (default: `1`)<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`                    |
| `.top`<br>`.dalao` | `.top [region] <shipName>` | `.top Yamato` <br> `.top NA Slava` | Show the top 15 players in that ship in the region <br> from https://wows-numbers.com/ |
| `.clan` | `.clan [region] <clanName>` | `.clan me` <br> `.clan PANTS` <br> `.clan eu TCL` | Show the clan's overview & stats |
| `.clan season` | `.clan [region] <clanName> <season>` | `.clan me S15` <br> `.clan PANTS S14` <br> `.clan eu TCL 15` | Show the clan members' clan battle stats in particular season |


### Others general commands
| Command name     | Usage                                        | Example                                                     | Description |
|------------------|----------------------------------------------|-------------------------------------------------------------|-------------|
| `.setwowsregion` | `.setwowsregion <region>`                    | `.setwowsregion na`                                         | Set the default region for this server<br>Valid values for `<region>`: `asia` (default), `na`, `eu`, `ru` <br>***Requires server admin permissions*** |
| `.wowsregion`    | `.wowsregion`                                | `.wowsregion`                                               | Get the default region for this server |
| `.uid` | `.uid <player>` | `.uid me`<br>`.uid B2U`<br>`.uid asia B2U` | Get the player's UID |
| `.clanuid` | `.clanuid [region] <clanName>` | `.clanuid me`<br>`.clanuid PANTS`<br>`.clanuid eu TCL` | Get the clan's UID |
| `.bonus`<br>`.code` | `.bonus <codes>` | `.bonus CODE1`<br>`.bonus CODE1 CODE2` | Generate a link to redeem one or more bonus codes |
| `.invite`  |  |  | Invite me to another server    |
