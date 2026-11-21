# Mattofessor

[English](README.md) · [简体中文](README.zh-CN.md) · **繁體中文** · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · [Français](README.fr.md) · [Русский](README.ru.md)

一款在 Windows 和 macOS 本機執行的英雄聯盟助手，可以取代 Porofessor：不需要 Overwolf，沒有廣告，也不用註冊帳號。支援繁體中文、簡體中文、英文、日文、韓文、西班牙文、法文和俄文。

> 本頁介紹如何安裝和使用。開發、建置和發佈的說明請見英文版 [README](README.md)。

- **個人資料**：你自己（或任何 Riot ID）的單／雙排和彈性積分牌位、過去的牌位、標籤、常用英雄、位置、專精和對戰紀錄。每場都能看 10 名玩家的完整資料：裝備和金錢，或輸出、承受、減免傷害和治療量。
- **當前對局**：英雄選擇階段偵查己方隊友，開局後偵查所有玩家（競技場也可以）。每名玩家顯示：牌位和最高牌位、是否疑似分身帳號、這隻英雄的場數、勝率、KDA 和專精、最近 5 場的 KDA，以及各種標籤（絕活玩家、強勢英雄、勝率、最常玩的英雄類型、預組等）。大混戰和隨機單中只看這類對戰本身，不套用召喚峽谷的標籤（位置、補兵、死亡數）。
- **遊戲內面板**：`Ctrl+Shift+M` 在遊戲畫面上顯示或隱藏面板（刻意避開 Porofessor 使用的 `Ctrl+X`，兩者可以同時執行）。載入畫面時面板會自己出現，對局開始後自動隱藏。`Ctrl+Shift+N` 開關敵方召喚師技能計時（對方使用技能時點一下）。這兩個快速鍵只在對局中有效。
- **版本強勢**：召喚峽谷各位置的梯隊榜（勝率、選用率、禁用率），以及大混戰的英雄和海克斯強化勝率，可依牌位區間篩選。
- **出裝**：符文和屬性碎片、召喚師技能、技能加點、起始裝備、鞋子和裝備，召喚峽谷依位置區分，隨機單中另有一套。一鍵匯入客戶端，也可以在鎖定英雄時自動匯入。大混戰的出裝頁還會列出這隻英雄的海克斯強化排名。
- **大混戰的海克斯強化**：英雄選擇和對局中，依稀有度列出這隻英雄該選和該避開的強化。選擇強化的卡牌出現時，每張卡上方會顯示你這隻英雄搭配這張卡的勝率，並標出最好的一張。卡牌內容用 Windows 內建的文字辨識讀取，依客戶端語言或英文辨識，不會儲存任何畫面。
- **版本更新**：最近 5 個版本的改動，召喚峽谷和大混戰分開顯示，內容來自 Riot 官方的繁體中文公告。
- **評分**：每場每名玩家都有 0.0 到 10.0 的表現評分和名次（第 1 到第 10）。勝方最高分是 MVP，敗方最高分是 SVP。

## 安裝

1. 打開本儲存庫的 **Releases** 頁面（GitHub 頁面右側），在最新版本裡下載 `Mattofessor_x.y.z_x64-setup.exe`。不要點「Download ZIP」，那是原始碼。
2. 執行安裝程式。因為安裝程式沒有程式碼簽章，Windows 可能會顯示「Windows 已保護您的電腦」：點 **其他資訊**，再點 **仍要執行**。安裝需要系統管理員權限，會安裝到 `C:\Program Files\Mattofessor`，並建立桌面捷徑。
3. 打開英雄聯盟並登入，Mattofessor 會自己連上客戶端。它預設開機啟動，可以在設定裡關閉。
4. 選用：申請你自己的免費 Riot API 金鑰，就能看到準確的積分勝敗場數。路徑是 設定 > Riot API 金鑰（可選）> **圖文分步教程**。每個人使用自己的金鑰，不要把你的分享給別人。

需要 Windows 10 或 11（缺少 WebView2 時會自動安裝）。遊戲要使用 **無邊框** 或 **視窗** 模式，遊戲內面板才能顯示在遊戲畫面上。快速鍵：`Ctrl+Shift+M` 面板，`Ctrl+Shift+N` 敵方技能計時，都可以在設定裡更改。解除安裝請到 Windows 設定 > 應用程式。設定和快取儲存在 `%APPDATA%\Mattofessor`。

### 防毒軟體誤判

有些防毒軟體可能會封鎖或隔離安裝程式，判定為「木馬」或「盜號程式」。這是誤判，原因有兩個：一是安裝程式沒有程式碼簽章，二是這個應用程式確實會讀取英雄聯盟客戶端本機的登入檔（lockfile）和螢幕畫面，而這正是防毒軟體特別注意的行為。請只從本儲存庫的 Releases 頁面下載，然後在防毒軟體裡還原被隔離的檔案，並把它和 `C:\Program Files\Mattofessor` 加入信任清單。

### macOS

1. 從最新版本下載 `Mattofessor_x.y.z_universal.dmg`（Apple 晶片和 Intel 通用），打開後把 Mattofessor 拖到「應用程式」。
2. 想在增幅裝置卡片上看到勝率，請在 **系統設定 > 隱私權與安全性 > 螢幕與系統錄音** 中允許 Mattofessor（設定 > 增幅裝置識別裡有按鈕），然後重新啟動 Mattofessor。

需要 macOS 13 或更新版本（卡片勝率需要 macOS 14）。Mattofessor 常駐選單列；設定儲存在 `~/Library/Application Support/Mattofessor`，Riot 金鑰儲存在鑰匙圈中。

## 更新

Mattofessor 會在背景檢查本儲存庫的最新版本（不需要 GitHub 帳號）：每 30 分鐘檢查一次，打開視窗或切回視窗時也會檢查。有新版本時會顯示提示列，點 **立即更新** 並允許 Windows 的提示，它就會重新啟動到新版本。對局中不會更新。可以在 設定 > 更新 裡關閉自動檢查，或點 **檢查更新** 手動檢查。

## 語言

設定 > 語言 裡可以選擇一種語言，也可以跟隨英雄聯盟客戶端。每種語言只顯示該語言的文字：英雄、裝備和強化的名稱與說明、規則、版本公告都來自 Riot 在該語言的官方資料。還沒有翻譯的內容會隱藏並顯示筆數，不會混入其他語言的文字。

## 選用：Riot API 金鑰

英雄聯盟客戶端會隱藏其他玩家的積分敗場。填上 Riot API 金鑰後，每名玩家都能看到準確的積分勝敗場數和勝率，分身帳號的判斷也會更準。在 [developer.riotgames.com](https://developer.riotgames.com/) 用 Riot 帳號登入，選擇 Register Product > Personal API Key：僅限個人使用，送出後立即通過，而且不會過期。控制台首頁的 Development API Key 24 小時就失效，所以應用程式裡的教學不使用它。金鑰以 Windows DPAPI 加密儲存在本機，介面上不會顯示。

## 資料來源

- 已登入的英雄聯盟客戶端的本機介面（LCU）和遊戲的 Live Client Data API。名稱和圖示來自 Riot 公開的 Data Dragon。沒有我們自己的伺服器。
- 召喚峽谷的梯隊榜和出裝：OP.GG 的公開介面（[opgginc/opgg-mcp](https://github.com/opgginc/opgg-mcp)）。
- 大混戰的勝率：海克斯大乱斗统计数据（[czgreat/aramhex-data](https://github.com/czgreat/aramhex-data)），採用 [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) 授權，資料來自中國伺服器的數百萬場對戰。Mattofessor 會在顯示這些數據的地方註明出處，並且永久免費（非商業）。
- 大混戰的英雄調整：[英雄聯盟 Wiki](https://wiki.leagueoflegends.com/en-us/ARAM:_Mayhem)（CC BY-SA 3.0）。強化的名稱和說明來自 Riot 的遊戲資料（透過 [CommunityDragon](https://communitydragon.org)）。
- 版本公告：Riot 官方各語言公告。

## 授權

程式碼採用 [MIT 授權](LICENSE)。顯示的資料各自沿用原本的授權：AramHex 的資料是 CC BY-NC 4.0，英雄聯盟 Wiki 的內容是 CC BY-SA 3.0，Riot 的遊戲資料和版本公告歸 Riot 所有。

## 法律聲明

Mattofessor 未經 Riot Games 認可，不代表 Riot Games 或任何正式參與製作、管理 Riot Games 旗下產品人員的觀點或意見。Riot Games 及其所有相關產品是 Riot Games, Inc. 的商標或註冊商標。
