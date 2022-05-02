![](https://cdn.discordapp.com/attachments/865923985891524638/936154921991041024/unknown.png)

[English](https://github.com/B-2U/ISAC---wws-stats-bot/blob/main/README.md) | **中文** | [日本語](https://github.com/B-2U/ISAC---wws-stats-bot/blob/main/README_ja.md)

## 戰艦世界戰績查詢機器人

想把機器人加進你的伺服器? 點 [這裡](https://discord.com/api/oauth2/authorize?client_id=961882964034203648&permissions=0&scope=bot%20applications.commands)
有其他問題? 加入我們的伺服器 [這裡](https://discord.gg/z6sV6kEZGV)

---

## 指令列表

### **❗ 被 `[]` 包住的參數是可以省略的**

#### 下面表格中的 `<player>` 可以是:  
  | valid values | 範例 |  
  |-|-|
  | `[region] <ign>` | `.wws asia B2U` or `.wws B2U` |  
  | `<@mention>` | `.wws @B2U#0900` |  
  | `<discordUserID>` | `.wws 930855839961591849` |  
  | `me` | `.wws me` |

| 指令名稱     | 用法                                        | 範例                                                    | 指令用途 |
|------------------|----------------------------------------------|-------------------------------------------------------------|-------------|
| `.link`          | `.link <地區> <遊戲ID>` | `.link asia B2U`      | 把你的Discord和遊戲帳號綁定<br>綁定後所有指令中可以用 `me` 取代 `[地區] <遊戲ID>`  |
| `.wws`           | `.wws <player>`                        | `.wws asia B2U`<br>`.wws B2U`<br>`.wws me`                  | 顯示該玩家的總戰績 |
| `.wws ship`      | `.wws <player> <船名> [戰鬥模式]`                 | `.wws asia B2U Ise`<br>`.wws B2U Yamato`<br>`.wws me Slava rank` | 顯示該玩家指定戰艦的戰績<br>`[戰鬥模式]`: `pvp` (預設), `solo`, `div2`, `div3`, `rank`          |
| `.recent`        | `.recent <player> [戰鬥模式] [天數]` | `.recent asia B2U`<br>`.recent B2U 7`<br>`.recent me rank 21`     | 顯示該玩家最近X天的戰績<br>`[天數]`: `1`~`21` (預設: `1`)<br>`[戰鬥模式]`: `pvp` (預設), `solo`, `div2`, `div3`, `rank`                    |
| `.top`<br>`.dalao` | `.top [地區] <船名>` | `.top Yamato` <br> `.top NA Slava` | 顯示該地區中這艘戰艦PR排行榜前15名的玩家數據 <br> 來自 https://wows-numbers.com/ |
| `.clan` | `.clan [地區] <公會名>` | `.clan me` <br> `.clan PANTS` <br> `.clan eu TCL` | 顯示該公會的平均戰績和公會戰紀錄 |
| `.clan season` | `.clan [地區] <公會名> <賽季>` | `.clan me S15` <br> `.clan PANTS S14` <br> `.clan eu TCL 15` | 顯示該公會成員們在指定公會戰賽季的戰績 |


### 其他指令
| 指令名稱    | 用法                                        | 範例                                                     | 指令用途 |
|------------------|----------------------------------------------|-------------------------------------------------------------|-------------|
| `.setwowsregion` | `.setwowsregion <region>`                    | `.setwowsregion na`                                         | 更改這個伺服器預設的地區 `<地區>`: `asia` (預設), `na`, `eu`, `ru` <br>***需要伺服器管理員權限*** |
| `.wowsregion`    | `.wowsregion`                                | `.wowsregion`                                               | 顯示這個伺服器目前預設的地區 |
| `.uid` | `.uid <player>` | `.uid me`<br>`.uid B2U`<br>`.uid asia B2U` | 顯示該玩家的 UID |
| `.clanuid` | `.clanuid [region] <clanName>` | `.clanuid me`<br>`.clanuid PANTS`<br>`.clanuid eu TCL` | 顯示該公會的 UID |
| `.bonus`<br>`.code` | `.bonus <codes>` | `.bonus CODE1`<br>`.bonus CODE1 CODE2` | 為一個或多個獎勵代碼生成官網兌換的連結 |
| `.invite`  |  |   | 機器人邀請連結    |
