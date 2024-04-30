![](https://i.imgur.com/YT4ZlZc.png)

[English](https://github.com/B-2U/ISAC/blob/master/README.md) | [中文](https://github.com/B-2U/ISAC/blob/master/README_zh.md) | **日本語**

## A World of Warships stats discord bot

自分のサーバに追加したい？こちら [here](https://discord.com/api/oauth2/authorize?client_id=961882964034203648&permissions=51264&scope=bot%20applications.commands)
をクリックもしくは私たちのサーバーに参加はこちら [Here](https://discord.gg/z6sV6kEZGV)

---

## コマンドリスト

### **❗ `[]` に入ってる数字は省略できます**

| コマンド                   | 使い方                                            | 入力例                                                                             | 説明                                                                                                                                                                             |
| -------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/link`                    | `/link`                                           | `/link`                                                                            | WOWSのアカウントとリンクします。<br>あとから `me` を入力しれば自分の戦績が反映されます                                                                                           |
| `.wws`                     | `.wws [region] <ign>`                             | `.wws asia B2U`<br>`.wws B2U`<br>`.wws me`                                         | プレイヤーの戦績照会                                                                                                                                                             |
| `.wws ship`                | `.wws [region] <ign> <shipName> [battleType]`     | `.wws asia B2U Ise`<br>`.wws B2U Yamato`<br>`.wws me Slava rank`                   | プレイヤーの特定の船を調べることができます <br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`                                                                   |
| `.recent`                  | `.recent [region] <ign> [battleType] [days]`      | `.recent asia B2U`<br>`.recent B2U 7`<br>`.recent me rank 30`                      | プレイヤーの一定期間の戦績を調べることができます <br>`[days]`: `1`~`30`(`90` for premium user) (default: `1`)<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank` |
| `.recent ship`             | `.recent <player> <shipName> [battleType] [days]` | `.recent asia B2U Z42`<br>`.recent B2U Halford 7`<br>`.recent me Kitakaze rank 30` | Show player's recent stats of a particular ship<br>`[days]`: `1`~`30`(`90` for premium user) (default: `1`)<br>`[battleType]`: `pvp` (default), `solo`, `div2`, `div3`, `rank`   |
| `.top`<br>`.dalao`         | `.top [region] <shipName>`                        | `.top Yamato` <br> `.top NA Slava`                                                 | トップ15のプレイヤーを調べることができます <br> from https://wows-numbers.com/                                                                                                   |
| `.server_top` <br> <.stop> | `.server_top <shipName>`                          | `.server_top Yamato`                                                               | Top 15 players in the ship in the discord server (min battles = 10)                                                                                                              |
| `.clan`                    | `.clan [region] <clanName>`                       | `.clan me` <br> `.clan PANTS` <br> `.clan eu TCL`                                  | クランの戦績を調べることができます                                                                                                                                               |
| `.clan season`             | `.clan [region] <clanName> <season>`              | `.clan me S15` <br> `.clan PANTS S14` <br> `.clan eu TCL 15`                       | Show the clan members' clan battle stats in particular season                                                                                                                    |


### Others general commands
| Command name        | Usage                          | Example                                                | Description                                                                                        |
| ------------------- | ------------------------------ | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| `/wows-region`      | `/wows-region`                 | `/wows-region`                                         | Set the default region for this server <br> ***Setting region requires server admin permissions*** |
| `/map`              | `/map`                         | `/map`                                                 | getting the specific map image                                                                     |
| `/roulette`         | `/roulette`                    | `/roulette`                                            | randomly pick ships for you                                                                        |
| `.uid`              | `.uid [region] <ign>`          | `.uid me`<br>`.uid B2U`<br>`.uid asia B2U`             | Get the player's UID                                                                               |
| `.clanuid`          | `.clanuid [region] <clanName>` | `.clanuid me`<br>`.clanuid PANTS`<br>`.clanuid eu TCL` | Get the clan's UID                                                                                 |
| `.bonus`<br>`.code` | `.bonus <codes>`               | `.bonus CODE1`<br>`.bonus CODE1 CODE2`                 | Generate a link to redeem one or more bonus codes                                                  |
| `.invite`           |                                |                                                        | Invite me to another server                                                                        |
