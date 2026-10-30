# onley

<p align="center">
  <img src="logo.png" alt="onley logo" width="200">
</p>

本地文件去重工具。通过 MD5 建立文件索引，找出重复文件，支持交互式清理——可在单台机器上使用，也可跨多台机器协同工作。

网站：https://fdaio.github.io/onley/

## 功能特性

- 基于 MD5 的全目录树重复文件检测
- 索引以 SQLite 文件形式存储在本地
- 断点续扫：重新扫描时，大小和修改时间未变的文件会被跳过
- 可配置的 goroutine worker 池并发扫描
- 持续建索引（`watch`）：文件新建、修改、删除时索引自动跟进
- 交互式清理（`clean`）或自动清理（`clean-all`）
- NSGo 桌面客户端，支持多文件夹扫描与清理（预览）
- 多机模式：replica 将本地索引与 master 对比，删除本地冗余副本，或将独有文件通过 HTTP 迁移到 master

## 安装

按平台下载 release 归档，或使用[下载页面](https://fdaio.github.io/onley/download.html)：

`<版本号>` 是不带前导 `v` 的 release 编号，例如 `0.2.10`。

```sh
# macOS（Apple silicon）
tar xzf onley_v<版本号>_darwin_arm64.tar.gz

# Linux（x86-64）
tar xzf onley_v<版本号>_linux_amd64.tar.gz

# macOS 桌面客户端预览（Apple silicon）
unzip onley-desktop_v<版本号>_macos_arm64.zip
open Onley.app

# macOS 桌面客户端 Homebrew 安装
brew tap laotree/tap
brew install --cask onley
```

发布范围为 darwin 与 linux 的 amd64 / arm64，以及 windows 的 amd64。
所有 release 资产共用一个 `checksums.txt`。桌面客户端预览同时发布
macOS arm64/x64、Windows x64 与 Linux x64。见
[releases](https://github.com/fdaio/onley/releases)。

`go install` 不可用，也没有提供：module 路径是 `onley`，不是一个可解析的仓库位置。
要修就得改每个包的 import 路径，因此没有做。

从源码构建：

```sh
git clone https://github.com/fdaio/onley
cd onley
make build      # 生成 ./onley
```

## 桌面客户端（预览）

macOS 可通过 Homebrew 安装：

```sh
brew tap laotree/tap
brew install --cask onley
```

NSGo 桌面客户端覆盖日常清理闭环：扫描一个或多个文件夹、查看重复组、每组选择一个
保留副本、确认后删除其余副本。扫描时显示实时 MB/s 速度仪表与动态指针。左侧栏保留
源文件夹；可把文件夹拖到拖放区，或点击拖放区输入路径。若某个文件夹位于另一个已选
文件夹内，会先归并到父目录再扫描；归并按绝对路径判断，不解析符号链接别名。顶部显示
已索引文件、重复组和可释放空间；标签栏按文件类型过滤，搜索按文件名或路径过滤。底部
栏是自动选择策略：保留最新、保留最旧或按路径保留。每组卡片显示渐变预览、文件标签和
每个副本的完整路径；原始副本是路径排序的第一个。清除会撤销全部保留选择，检查并删除
会打开确认框。该策略不会绕过确认直接删除。它和 CLI 共用 `~/.onley/local.db` 索引。
macOS 发布包使用 Developer ID 签名并已公证；本地 `make app` 构建未签名。Windows 代码
签名正在申请中，Linux 包仍未签名，打开前请阅读下方说明。

macOS 构建并打开：

```sh
make app
# Apple silicon
open cmd/onley-app/dist/Onley-0.2.10-macos_arm64/Onley.app
# Intel
open cmd/onley-app/dist/Onley-0.2.10-macos_x64/Onley.app
```

构建应用需要 Go 1.27.3 和 Xcode；NSGo 版本固定在 `go.mod`。
NSGo 仍处于 early 阶段、没有稳定 API，升级应作为独立任务。
`make app` 构建当前 Mac 架构；`make app-all` 打包 macOS、Windows 和 Linux。
本轮以 macOS 为运行验证目标；Windows 与 Linux 已验证可构建、可打包，但尚未运行验证。

客户端使用黑白+粉色+浅绿的 brutalist 配色，只做桌面端。界面支持 English 与
简体中文，首次启动跟随系统语言；左侧栏底部的 EN/中文按钮可切换，选择会被记住。
文案位于 `internal/i18n`，新增语言目录即可扩展。

手机与 Web 客户端需要 onley 提供本地服务 API，本预览不做。
客户端不支持 `watch`、replica、`config` 和自定义 `-db`，这些继续用 CLI。
扫描路径需要手动输入，暂未提供原生目录选择器。扫描与删除只操作默认
索引，删除不可撤销。删除前会校验文件大小和修改时间；旧版本建立的索引需要
重新扫描一次才能记录 mtime。Onley 不会删除正在运行的自身程序；如果自身出现
在重复组中，它只能作为保留副本，Windows 也会锁定运行中的 exe。
Windows 与 Linux 的原生菜单栏默认隐藏，按 Alt 显示；它由系统绘制，不跟随应用
配色。macOS 的顶部菜单栏跟随系统外观，切换深色模式后与 Midnight Terminal
窗口一致。macOS 发布包已签名并公证，首次打开不再需要右键选择“打开”；本地
`make app` 构建未签名。Windows 代码签名正在申请中，Linux 包仍未签名。
Windows 上 SmartScreen 可能提示风险，Smart App Control 可能阻止未签名应用；
Linux 包打开前请核对 release checksum。第三方声明见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

无头测试随仓库测试一起跑；macOS 原生 UI 测试需要 Xcode：

```sh
go test ./...
cd cmd/onley-app && go tool nsgo test -target macos
```

## 代码签名政策

Windows 代码签名正在申请中，当前 Windows 二进制尚未签名。项目的[代码签名政策](CODE_SIGNING.md)
说明了计划采用的 SignPath Foundation 流程及覆盖范围。

## 快速上手

```sh
# 扫描目录，建立索引
onley scan ~/Downloads

# 查看重复文件
onley dupes

# 交互式选择保留哪些文件
onley clean

# 或自动保留每组第一个文件，删除其余
onley clean-all

# 查看统计信息
onley stats

# 或在文件变化时自动维护索引
onley watch ~/Downloads
```

## 子命令

| 命令 | 说明 |
|---|---|
| `scan <dir>` | 扫描 `dir` 并对每个文件计算 MD5 建立索引 |
| `watch <dir>` | 随 `dir` 下文件的变化持续维护索引 |
| `dupes` | 列出所有重复文件组 |
| `clean` | 交互式：逐组选择要保留的文件 |
| `clean-all` | 每组保留第一个文件、删除其余（需一次确认） |
| `stats` | 显示总文件数和重复文件数 |
| `serve` | 启动 master HTTP 服务（用于多机模式） |
| `replica check` | 将本地索引与 master 比对，生成并执行计划 |
| `config set` / `config show` | 读写按用户保存的设置 |

## 全局参数

| 参数 | 默认值 | 说明 |
|---|---|---|
| `-db <path>` | `~/.onley/local.db` | SQLite 数据库文件路径 |
| `-workers <n>` | `max(1, CPU数-1)` | 并发 hash worker 数量 |
| `-y`、`--yes` | 关闭 | 对确认一律回答 yes |

选项要放在子命令之前：`onley -y clean-all`，而不是 `onley clean-all -y`。

`-y` 是「照计划执行」的许可，不是「随意销毁」的许可。在多机模式下，只有当本次运行
已经把该内容上传到 master 时，它才授权删除本地副本；master 早就有的文件一律保留、
在 stderr 警告，并以非 0 退出，因为有活留给人。详见[多机模式](#多机模式replica)。

默认索引放在 home 目录下，这样任何目录看到的是同一份索引：在 `~/Downloads`
里 `onley scan`，换个目录再 `onley stats`，仍然能看到扫描结果。该目录不存在
时会自动创建。

如果当前目录下有 `onley.db`、而默认索引还不存在，onley 会把它复制过去并给出
提示。旧文件保留在原地，用 `-db ./onley.db` 仍可继续使用。

显式指定的 `-db` 路径按原样使用，相对路径相对于当前目录。onley 不会为它创建
缺失的父目录，因此路径写错时会直接报错，而不是悄悄建出一个目录。

## 扫描

```sh
onley scan /Volumes/data
onley -workers 4 scan /Volumes/data
```

扫描进度按 worker 逐行显示。对同一目录进行第二次扫描时，大小和修改时间未变的文件会被跳过，中断后续扫的开销很低。

## 持续监听

`watch` 会自动维护索引，不需要反复手动扫描：

```sh
onley watch ~/Downloads
onley watch -debounce 2s /Volumes/data
```

| 参数 | 默认值 | 说明 |
|---|---|---|
| `-debounce <dur>` | `500ms` | 文件停止变化多久后才纳入索引 |
| `-master <url>` | 未配置时为无 | 把新文件推给这台 master；未指定时使用 `ONLEY_MASTER` 或 `~/.onley/config.json`。三处都没有时只维护索引 |
| `-interval <dur>` | `30s` | 推送队列的间隔 |

它和 `scan` 一样覆盖整棵目录树，并在启动时为已存在的文件建立索引；运行期间新建的子目录也会自动纳入。

`watch` 只维护索引。文件被删除或改名移走后，索引里对应记录会移除，但磁盘上的文件不会被删除 —— 清理仍然是 `clean` 和 `clean-all` 的职责。

文件在 `-debounce` 窗口内不再变化后才会纳入索引，关窗时还会再核对一次大小和修改时间，所以正在写入的文件不会被算进半截内容。如果你的文件是分段写入、段间停顿较久，请调大 `-debounce`：停顿超过窗口的写入方，从索引器看来和已经写完没有区别。

`watch` 运行期间可以用其它命令访问同一个索引。按 Ctrl-C 停止。

### 推送到 master

给了 master（来自 `-master`、`ONLEY_MASTER` 或 `~/.onley/config.json`）之后，
`watch` 会把收录到的文件也上传上去：

```sh
onley watch -master http://master-host:8080 ~/Downloads
```

**它只上传，绝不删除。** 新文件必然走 `replica check` 的 migrate 分支，而那一支
是「上传后删掉本地副本」—— 所以会同步的 watch 会把它被要求监听的目录清空。
本地副本在这里保留，删除仍然只在 `replica check` 里被提出并确认。

代价要说清楚：master 存一份，**本地那份也还在**，所以磁盘占用是增加而不是减少。
这是推送，不是 `replica check` 那种整合。想要空间回来，就主动跑 `replica check`。

具体行为：

- 队列是上次推送以来收录到的内容。启动时那就是整棵已有的目录树，因为启动扫描
  会把它们都报一遍 —— 所以第一轮就是那次「补上之前漏掉的部分」的对账。
- 内容相同的文件只查询一次、只上传一次，不是每个文件各来一遍。
- master 已经有了的内容不会重复上传，也不会被删除。
- 一轮失败后，下一轮的等待时间翻倍，上限是间隔的十六倍；成功一轮就恢复原值。
  挂了一个星期的 master 不会被每 30 秒戳一次。
- 连不上 master 不会影响建索引。

上传跑在独立的 goroutine 上。这不是可选项：watcher 的事件通道有界，满了会丢弃
事件，所以上传如果跑在索引循环里，代价不是「变慢」而是**静默漏掉索引条目**。

## 清理

`clean` 逐组展示重复文件，并询问要保留哪些。输入单个编号、逗号分隔的多个编号，或直接回车跳过该组。

```
── Group 1  MD5: d8e8fca2dc0f896fd7cb4cb0031ba249  size: 4.0 KB ──
  [1] /Users/alice/docs/report.pdf
  [2] /Users/alice/backup/report.pdf
Keep number(s) (e.g. 1 or 1,2; Enter to skip): 1
```

`clean-all` 每组按路径字母顺序保留第一个文件，一次确认后删除其余文件。

`clean` 与 `clean-all` 按索引快照执行，删除前不会重新校验文件大小或修改时间；
如果文件可能已变化，请先重新 `scan`。桌面客户端会在删除前校验。

`-y` 跳过这次确认，也就是让 `clean-all` 能在 cron 里 stdin 为空时无人值守地跑：

```sh
onley -y clean-all
```

`-y` 不会代替 `clean` 的逐组选择。「每组保留哪一个」正是这个命令存在的理由，替你回答
就等于选了一个和你想要的不一样的删除目标。所以 `onley -y clean` 仍然读 stdin 来收集
选择，只跳过最后那一次 yes/no。

不带 `-y` 时，stdin 读到 EOF 会直接失败，而不是悄悄地把所有文件都留着 —— 空输入流
并没有拒绝任何东西。答 `n` 是一个决定，视为成功。

## 多机模式（Replica）

多机模式允许多台机器将各自的独有文件统一汇聚到一台 master 上。

多机模式没有认证，也没有 TLS。只应在可信网络中运行 `serve`，或把它放在反向代理、
VPN 或防火墙之后。

**在 master 机器上**，启动 HTTP 服务：

```sh
onley -db /data/master.db serve -port 8080 -store /data/files
```

| 参数 | 默认值 | 说明 |
|---|---|---|
| `-port <n>` | `8080` | 监听端口 |
| `-store <dir>` | `onley-store` | 接收迁移文件的存储目录 |

**在每台 replica 机器上**，先把 master 记下来，再扫描并比对：

```sh
onley config set master http://master-host:8080
onley scan /my/files
onley replica check
```

`replica check` 必须知道连哪台 master，而这个值每台机器是固定的，所以记在索引旁边的
`~/.onley/config.json` 里。单次运行仍然可以覆盖：

| 来源 | 示例 | 适用场景 |
|---|---|---|
| `-master` | `onley replica check -master http://other:8080` | 临时换一台 master 跑一次 |
| `ONLEY_MASTER` | `ONLEY_MASTER=http://other:8080 onley replica check` | cron 里指向另一台，不必改文件 |
| `~/.onley/config.json` | `onley config set master http://host:8080` | 常规用法 |

三者按此顺序取第一个非空值，所以 `-master ""` 与不传等价。`replica check` 会打印
它最终用的 master 以及来自三者中的哪一个 —— 配置里存了过期 URL 时完全看不出来，失败
表现会和「master 连不上」混在一起。

`config set` 会拒绝不是 `http://` 或 `https://` 的 scheme、缺 host 的 URL，也会拒绝
它不认识的键，这样打错字在写入的那一刻就被拦住，而不是等到下一次 replica 运行时
才发现。配置文件无法解析时命令直接报错退出并指出文件路径，不会当成空配置 —— 在多机
模式下，连错 master 的后果是删文件。

`replica check` 对本地索引中的每个文件查询 master，生成操作计划：

- **删除本地副本** — master 已有该 MD5，本地文件冗余。
- **迁移到 master** — master 没有该文件，将上传至 master 后删除本地副本。

执行前会完整展示计划，输入 `y` 确认，按回车或输入 `n` 取消。

`-y` 会无人值守地执行计划，但有一个例外：上传照常进行，本次已上传到 master 的那些
本地副本会被删除 —— 因为删之前内容已经在两处。而 **「删除本地副本」** 那一类会被跳过：
master 本来就有那份内容，如果删除结果证明是错的，就什么都留不下了。它们仍然列在计划里，
onley 在 stderr 上警告，并以非 0 退出。

```sh
$ onley -y replica check
Delete locally (already on master, 892 file(s)):
  /my/files/photo_001.jpg
  ...

warning: 892 file(s) are already on the master and were left in place.
         -y does not delete a local copy the master already has, because nothing
         would be left anywhere if that turned out to be wrong.
         The list above is what is waiting for a decision: delete them yourself,
         or drop -y and confirm the plan interactively.

Done: 0 deleted, 342 migrated, 0 failed.
$ echo $?
1
```

要让定时任务也删掉它们，请先过一遍那份清单。去掉 `-y` 并输入 `y` 是交互路径，
仍然会删除计划里的全部内容。

```
Comparing 1 234 file(s) with master...

Delete locally (already on master, 892 file(s)):
  /my/files/photo_001.jpg
  ...

Migrate to master (not on master, 342 file(s)):
  /my/files/project_final_v3.zip
  ...

Proceed with the above? [y/N]
```

迁移文件通过 HTTP multipart 上传，master 以内容寻址方式存储（路径格式：`<store>/<md5前2位>/<md5后30位>/<文件名>`）。上传成功后，replica 本地文件及其索引记录均被删除。

## Master HTTP API

master 对外暴露一组小型 JSON API，供 `replica check` 内部调用，也可直接访问。
该 API 没有认证，也没有 TLS；见上方部署注意事项。

| 方法 | 路径 | 说明 |
|---|---|---|
| `GET` | `/v1/health` | 返回 `{"ok":true}` |
| `GET` | `/v1/check?md5=<hash>` | 返回 `{"found":bool,"paths":[...]}` |
| `POST` | `/v1/ingest` | Multipart 上传，字段：`file`、`md5` |

`onley` 发送这个请求时使用 chunked 传输编码：文件是流式送出而不是先测长再缓冲，
因此请求里没有 `Content-Length`。直接调用这个 API 的客户端不应当假定
`Content-Length` 一定存在。

## 开发

```sh
make test       # 运行所有测试
make coverage   # 生成覆盖率报告 → coverage.html
make lint       # golangci-lint
make clean      # 删除二进制文件和覆盖率文件
```

运行环境要求：Go 1.27.3 或更高（module 声明的版本）
