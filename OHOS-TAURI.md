# OHOS (HarmonyOS) Tauri 适配 — ohos-tauri 分支

artcraftx（Tauri 应用，`crates/artcraftx`）在 OpenHarmony 真机（HAD-W24 鸿蒙 PC）上运行的
适配分支。**真机首启链已贯通**：前端构建 → `cargo tauri ohos init` → HAP 构建+签名 → 装机
→ 启动 → 首屏渲染（深色工作台 UI 完整呈现）+ 前端 IPC 回环（`get_app_*_command`）+
主窗关闭干净退出。

配方与 artcraft 仓 `ohos-tauri` 分支同源（1:1 迁移 + 本仓特有差异，下文以差异为主）。

> 2026-11-09 重锁 fork `ohdev-git` 至 `73bffac6f`（自 `46a19d8d`）并全链重验：
> HAP 双 `.so` 齐、冷启/首屏（OnFirstScreenPaint 1552ms）/IPC 回环/关闭退出链全过。
> fork 新增 `sync_ohos_tauri_module`（libc++ 注入通道变更，见下）与主窗 close 修复。

## 前置依赖

| 依赖 | 说明 |
|---|---|
| Tauri OHOS fork | 兄弟团队仓（GitHub `Eulogizethesun` 系十仓，外发分支统一 `ohdev-git`），**只消费不修改**。根 `Cargo.toml` 的 `[patch.crates-io]` 16 条 git 重定向 + openh264-sys2 vendor path——cargo 自动拉取，**无需本地 tauri 源码树**。当前锁 `73bffac6f`（内部提速备选：换 path 指向 `../../tauri-3.0` checkout，版本面与 ohdev-git 分支一致） |
| tauri-cli（OHOS 版） | `cargo install --git https://github.com/Eulogizethesun/tauri --branch ohdev-git --locked tauri-cli`（crates.io 发行版无 OHOS 支持） |
| ohrs | `cargo install ohrs --version 1.3.1`（tauri-cli 内部调用它编 `.so`） |
| DevEco Studio | SDK clang（`CC/CXX_aarch64_unknown_linux_ohos`）、NDK `libc++_shared.so`、libclang（bindgen 用） |
| Node/Nx | 前端是 Nx monorepo（`frontend/`），需先构建 `frontend/apps/artcraft/dist` |
| OHOS 签名物料 | DevEco auto-sign 产出的 p12/cer/p7b + 口令（本机 `.ohos/config/`，不入库） |

## 前端构建（先于一切 cargo 命令）

tauri-codegen 嵌入 dist 时要求目录存在，缺失则任何 cargo 构建直接失败：

```bash
cd frontend
printf 'n\n' | npx nx run artcraft:build
```

`printf 'n\n'` 用于应答 Nx Console 的 enquirer 交互提示（CI/管道环境会卡死）。
产物在 `frontend/apps/artcraft/dist/`（tauri.conf.json `frontendDist` 指向之）。

## 本分支的源码改动（全量）

1. **依赖挂载**（根 `Cargo.toml` + `Cargo.lock`）：`[patch.crates-io]` 16 条 → fork 的
   `ohdev-git` 分支（git 依赖：tauri 2.10.3 / tauri-build 2.5.6 / tauri-plugin 2.5.4 /
   wry 0.55.0 / tao 0.35.0 / muda 0.19.1 / tray-icon 0.24.0 / window-vibrancy 0.7.1 /
   ability 1.0.0-beta.0 / dialog 2.7.1 等）+ openh264-sys2 vendor path 条目。
   **git 依赖模式已真机端到端验证**（2026-11-03：无 TAURI_WORKSPACE_ROOT、无本地 tauri 树
   全量构建 → 签名 HAP → 冷启/首屏/IPC 回环全过；fork 侧另有 examples/api 真机验证）。
   **锁漂移坑**：patch 生效与否不能靠 `cargo tree`，要 `grep -c '^\[\[patch.unused\]\]' Cargo.lock` = 0；
   source 形态切换（path↔git）后 `cargo update -p tauri` 会报 "did not match any packages"
   ——改用 `cargo fetch` 重写 lock 的 source 行（版本零漂移）。
1a. **dialog 插件 HAR 入仓**（`crates/artcraftx/plugins/dialog/openharmony/`，6 文件）：
   tauri-cli 找插件 OHOS HAR 只认三处——`<project_dir>/plugins/<name>/openharmony`（本方案）、
   plugins-workspace 兄弟目录上溯、`TAURI_WORKSPACE_ROOT` 环境变量。git 依赖模式下后两处
   不存在，dialog 的 HAR 拷入仓内即被自动发现；fs/log/upload/opener 无 OHOS HAR
   （构建时 Warn 跳过，属预期）。@tauri/app HAR 不用操心——随 tauri crate 的
   cargo git checkout 自动就位。
2. **openh264-sys2 vendor**（`crates/vendor/openh264-sys2`）：thumbnails 无条件依赖
   openh264 → openh264-sys2，其 build.rs 的 `TargetEnv` 解析不认 `env="ohos"`（直接
   panic，无 feature 可关）。本仓 vendor 副本仅加一个 `"ohos" => TargetEnv::NoEnv`
   match 臂（该枚举不参与任何构建决策），其余与上游逐字节同。`[patch.crates-io]` 挂第 16 条。
   （libwebp-sys2 同样走 bindgen，但无需改动。）
3. **入口条件**（`src/lib.rs`）：`cfg_attr` 加 `target_env = "ohos"`（OHOS 上 `cfg(mobile)`
   恒 false）。
4. **OHOS 数据根**（`src/lib.rs` `run()`）：`AppDataRoot` 改用 Ability `filesDir` +
   `"artcraftx"`（OHOS 沙箱无 `$HOME`，走 `UserDirs` 会 `.expect` 即启动 SIGABRT）。
5. **hilog 可见性**（`src/lib.rs`）：`forward_stdio_to_hilog()`（tag `RustStdoutStderr`）。
6. **cfg 垫片**（`src/utils/ohos_compat.rs` 新文件 + 7 个窗口文件 import）：
   `DesktopOnlyBuilderCompat` 给桌面专属 builder 方法提供 ohos 面空操作。
7. **ohos deps**（`crates/artcraftx/Cargo.toml`）：`openharmony-ability` / `napi-ohos` /
   `napi-derive-ohos` + `ohos-hilog-binding`（redirect 特性）。
   **坑**：`[target.'cfg(target_env = "ohos")'.dependencies]` 段必须放在文件里
   **vendor 两行（`tauri-plugin-http` / `tauri-plugin-opener`）之后**——TOML 靠表头分段，
   注释和空行不结束上一段；插在其前会把这两条 upstream 无条件依赖吞进 ohos 专属段，
   host/desktop 构建面直接损坏（依赖不在图里 → capabilities 引用的 `http:*` / `opener:*`
   权限解析不到 → app build.rs 报 "Permission http:allow-fetch not found"）。OHOS 面恰好
   不触发（cfg 匹配），属「只在对照平台显形」型回归。
8. **vendor 对齐**（`crates/vendor/tauri-plugin-http`）：tauri-plugin 2.5.4 对齐 + 权限
   schema 补 `openHarmony` 枚举值（重新自动生成）。
9. `tauri.ohos.conf.json`：identifier `com.egui.device-spike`（codegen 转 bundleName
   `com.egui.device_spike`，真机判据单槽）+ 空 `beforeBuildCommand`（前端手动构建，见上）。
10. `.gitignore`：`gen/` + `tauri.ohos.conf.local.json`。

## 构建配方

```bash
cd crates/artcraftx

CI=true SQLX_OFFLINE=true CMAKE_GENERATOR=Ninja BORING_BSSL_RUST_CPPLIB=c++ \
OHOS_DEVICE_TYPE=desktop \
LIBCLANG_PATH="<SDK>/openharmony/native/llvm/bin" \
BINDGEN_EXTRA_CLANG_ARGS='-isystem "<SDK>/openharmony/native/llvm/lib/clang/15.0.4/include" -isystem "<SDK>/openharmony/native/sysroot/usr/include/aarch64-linux-ohos" -isystem "<SDK>/openharmony/native/sysroot/usr/include"' \
CC_aarch64_unknown_linux_ohos="<SDK>/llvm/bin/clang.exe" \
CXX_aarch64_unknown_linux_ohos="<SDK>/llvm/bin/clang++.exe" \
OHOS_KEYSTORE_FILE=<p12> OHOS_KEYSTORE_PASSWORD=<口令> \
OHOS_KEY_ALIAS=<别名> OHOS_KEY_PASSWORD=<口令> \
OHOS_APP_CERT_FILE=<cer> OHOS_PROFILE_FILE=<p7b> \
cargo tauri ohos build --device-type desktop --ci -t aarch64
```

**bindgen 三 `-isystem` 是本仓刚需**（artcraft 没踩到）：boring-sys2 等 bindgen 面在 OHOS
交叉目标下，Windows libclang 的 resource-dir 解析静默失败 → `stddef.h` / `sys/types.h` /
`bits/alltypes.h` 逐个找不到。三个头文件路径分别对应 clang 内建、sysroot 主头、musl
multiarch 头。**带空格的路径值必须内层双引号**（BINDGEN_EXTRA_CLANG_ARGS 走 shell-words
解析，外层引号不够）。

### ohrs 幽灵目录坑（构建 panic）

boring-sys2 的 `emit_link_directives` 恒发两条 `cargo:rustc-link-search=native={out}/build/crypto/`
与 `.../build/ssl/`（上游自注 "pretty redundant"）——本机上这两个目录**从不存在**（cmake 把
libcrypto.a/libssl.a 放 out/build 根）。cargo 容忍缺失的 -L 目录，但 tauri 链路会 spawn
`ohrs build`，ohrs 的 artifact.rs 对 -L 目录做 canonicalize，**目录不存在即 panic**。
artcraft 当年靠其 out/build 恰好有空 `crypto/`+`ssl/` 目录蒙混过关；本仓需手工补：

```bash
mkdir -p target/aarch64-unknown-linux-ohos/release/build/boring-sys2-<hash>/out/build/{crypto,ssl}
```

两步法定位 `<hash>`（目录名含构建指纹，无法预知）：先跑一次 §构建配方，等 panic
报出完整缺失路径（`.../boring-sys2-<hash>/out/build/crypto/ ... not found`），照抄
该路径 mkdir 空目录后重跑。target 清理后需重补。

### 构建后必做：注入 libc++_shared.so（白屏根因）

`libartcraftx_app_lib.so` 因 `BORING_BSSL_RUST_CPPLIB=c++` 带 `DT_NEEDED
libc++_shared.so`，OHOS 不随包提供 → 缺库 = 模块加载失败 = 白屏。**注入点必须选
dialog 插件【源】目录**（本仓已带）：

```bash
cp "<SDK>/openharmony/native/llvm/lib/aarch64-linux-ohos/libc++_shared.so" \
   crates/artcraftx/plugins/dialog/openharmony/libs/arm64-v8a/
```

原理：tauri-cli 每次构建把该目录整树重拷到 `gen/ohos/dialog/`（只跳 `.tauri`/`target`），
HAR 模块根的 `libs/arm64-v8a/` 是 hvigor 自动打包约定——libc++ 随 HAR 打进 HAP，
对 tauri-build 同步行为免疫。复核：`unzip -l <hap> | grep libc++` 应见
`libc++_shared.so`（1,262,504 B）与 `libartcraftx_app_lib.so` 并存。

**`gen/ohos/tauri/libs/` 通道已失效**（2026-11-09 实证）：fork `73bffac6f` 起
tauri-build 的 `sync_ohos_tauri_module` 在**每次 Rust 构建**把 `gen/ohos/tauri`
镜像回 `crates/tauri/mobile/ohos` 模板并剪枝多余文件——git 依赖模式同样触发
（旧结论「git 模式无 sync」作废），注入即被清 = 装机白屏。`entry_desktop/libs/`
（`ohrs --dist` 同步删除）与 gen 侧 `dialog/libs/`（整树重拷覆盖）同样不可用。

### init 后必做的手工修正（gen/ 不入库，重建即重打）

**用 ohdev-git 版 tauri-cli（§前置依赖的安装命令）init 时，只剩两处**：

1. `libc++_shared.so` 注入（见上节 dialog 源目录通道）。
2. boring-sys2 幽灵目录（见上节两步法；本仓当前 hash
   `boring-sys2-81f8e7289d3c6aa4`，与 artcraft 的 `80aba92a579f68dd` 不同——
   依赖图差异所致，target 清理后重跑两步法即可）。

两处旧坑在 ohdev-git CLI（`46a19d8d` 起）已由模板根治，**新 init 不需要再做**：

- ~~ability.har 路径~~：模板改为 OHPM registry 形态 `@ylong-rs/ohrs-ability: 1.0.0-beta.1`
  （不再生成断链的 7 级 `file:` 相对路径；构建时 ohpm 从 registry 拉取）。
- ~~摘除 READ_PASTEBOARD~~：模板 `module.json5` 的 `requestPermissions` 已只剩
  INTERNET / SET_WINDOW_TRANSPARENT / WINDOW_TOPMOST / LOCK_WINDOW_CURSOR / PRINT，
  装机不再报 `9568289`。

仅当 `cargo install --list` 显示的 tauri-cli 来源**不是** `Eulogizethesun/tauri`（git）时
（crates.io 版或 10 月前的旧二进制），才会踩回这两坑：路径断链手工改
`file:` 指向本地 `openharmony-ability/ability.har`、`module.json5` 手工摘
READ_PASTEBOARD——换装 §前置依赖的 ohdev-git CLI 重新 init 即消除。

## 真机验证

`verify-launch.ps1`（仓根）：`bm uninstall -n` → `hdc install` → `hilog -r` → `aa start` →
pidof → hilog 启动链 → faultlog 检查 → `snapshot_display` 截图。判据（2026-11-03 全过；
同日转 ohdev-git git 依赖后同套复验全过，OnFirstScreenPaint 1162ms）：

- 冷启（`onCreate` 非 `onNewWant`）+ pid 存活 + `:gpu`/`:render` 子进程、无新 cppcrash
- hilog `RustStdoutStderr`：`OHOS filesDir base:` → `NativeAbility onCreate` →
  `scheme_handler register custom schemes tauri` → `NWebRenderHandler::OnFirstScreenPaint`
  （本机 1199ms）→ app 自有线程群（thumbnail_worker / main_window / storyteller / discord）
- 前端 IPC 回环：`get_app_info_command called...`（前端→Rust 命令链路活）
- 首屏截图非白（完整工作台 UI）；心跳：discord_presence_thread 10s 重连节拍持续即存活

注意：
- `bm uninstall` 用 **`-n`**（本设备 bm 构建不认 `-b`）。
- 槽内旧包版本号高时 `hdc install` 报 `9568263 install version downgrade`——先卸净。
- `aa start` 命中已在跑的实例时只会 `onNewWant` 拉前台（伪「启动成功」），验冷启先
  force-stop。
- `hilog -x` 混有 ArkWeb C++ 内部日志，过滤：
  `grep -vE 'audio_dump|oh_gin|nweb_handler|arkweb_browser'`。
