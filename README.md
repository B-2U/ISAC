![](https://i.imgur.com/YT4ZlZc.png)

**English** | [中文](https://github.com/B-2U/ISAC/blob/master/README_zh.md) | [日本語](https://github.com/B-2U/ISAC/blob/master/README_ja.md)

## A World of Warships stats discord bot

Want to add it to your server? Click [here](https://discord.com/api/oauth2/authorize?client_id=961882964034203648&permissions=51264&scope=bot%20applications.commands)
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
| `/link`          | `/link` | `/link`      | Link your wows account<br>Afterwards you can use `me` as a shortcut to `[region] <ign>`  |
| `.wws`           | `.wws <player>`                        | `.wws asia B2U`<br>`.wws B2U`<br>`.wws me`                  | Show player's stats overview |
| `.wws ship`      | `.wws <player> <shipName> [battleType]`                 | `.wws asia B2U Ise`<br>`.wws B2U Yamato`<br>`.wws me Slava rank` | Show player's stats of a particular ship<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`          |
| `.recent`        | `.recent <player> [battleType] [days]` | `.recent asia B2U`<br>`.recent B2U 7`<br>`.recent me rank 30`     | Show player's recent stats<br>`[days]`: `1`~`30`(`90` for premium user) (default: `1`)<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`                    |
| `.recent ship` | `.recent <player> <shipName> [battleType] [days]` | `.recent asia B2U Z42`<br>`.recent B2U Halford 7`<br>`.recent me Kitakaze rank 30`     | Show player's recent stats of a particular ship<br>`[days]`: `1`~`30`(`90` for premium user) (default: `1`)<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`                    |
| `.top`<br>`.dalao` | `.top [region] <shipName>` | `.top Yamato` <br> `.top NA Slava` | Show the top 15 players in that ship in the region <br> from https://wows-numbers.com/ |
| `.clan` | `.clan [region] <clanName>` | `.clan me` <br> `.clan PANTS` <br> `.clan eu TCL` | Show the clan's overview & stats |
| `.clan season` | `.clan [region] <clanName> <season>` | `.clan me S15` <br> `.clan PANTS S14` <br> `.clan eu TCL 15` | Show the clan members' clan battle stats in particular season |


### Others general commands
| Command name     | Usage       | Example        | Description |
|------------------|-------------|----------------|-------------|
| `/wows-region` | `/wows-region`| `/wows-region` | Check / Set the default region for this server <br> ***Setting region requires server admin permissions*** |
| `/map`         | `/map`        | `/map`         | getting the specific map image |
| `/roulette`    | `/roulette`   | `/roulette`    | randomly pick ships for you |
| `.uid`         |`.uid <player>`| `.uid me`<br>`.uid B2U`<br>`.uid asia B2U` | Get the player's UID |
| `.clanuid`     | `.clanuid [region] <clanName>` | `.clanuid me`<br>`.clanuid PANTS`<br>`.clanuid eu TCL` | Get the clan's UID |
| `.bonus`<br>`.code` | `.bonus <codes>` | `.bonus CODE1`<br>`.bonus CODE1 CODE2` | Generate a link to redeem one or more bonus codes |
| `.invite`  |  |  | Invite me to another server    |
