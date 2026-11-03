# OHOS (HarmonyOS) Tauri 适配 — ohos-tauri 分支

artcraftx（Tauri 应用，`crates/artcraftx`）在 OpenHarmony 真机（HAD-W24 鸿蒙 PC）上运行的
适配分支。**真机首启链已贯通**：前端构建 → `cargo tauri ohos init` → HAP 构建+签名 → 装机
→ 启动 → 首屏渲染（深色工作台 UI 完整呈现）+ 前端 IPC 回环（`get_app_info_command`）。

配方与 artcraft 仓 `ohos-tauri` 分支同源（1:1 迁移 + 本仓特有差异，下文以差异为主）。

## 前置依赖

| 依赖 | 说明 |
|---|---|
| Tauri OHOS fork | 兄弟团队仓，checkout 到 `../../tauri-3.0`（相对本仓根），**只消费不修改**。根 `Cargo.toml` 的 `[patch.crates-io]` 15 条 path 重定向 + openh264-sys2 vendor 已按此写死 |
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

1. **依赖挂载**（根 `Cargo.toml` + `Cargo.lock`）：`[patch.crates-io]` 15 条 → fork
   （tauri 2.10.3 / tauri-build 2.5.6 / tauri-plugin 2.5.4 等）。
   **锁漂移坑**：patch 生效与否不能靠 `cargo tree`，要 `grep -c '^\[\[patch.unused\]\]' Cargo.lock` = 0；
   漂了就 `cargo update -p tauri` 整族翻转。
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
8. **vendor 对齐**（`crates/vendor/tauri-plugin-http`）：tauri-plugin 2.5.4 对齐 + 权限
   schema 补 `openHarmony` 枚举值（重新自动生成）。
9. `tauri.ohos.conf.json`：identifier `com.egui.device-spike`（codegen 转 bundleName
   `com.egui.device_spike`，真机判据单槽）+ 空 `beforeBuildCommand`（前端手动构建，见上）。
10. `.gitignore`：`gen/` + `tauri.ohos.conf.local.json`。

## 构建配方

```bash
cd crates/artcraftx

CI=true SQLX_OFFLINE=true CMAKE_GENERATOR=Ninja BORING_BSSL_RUST_CPPLIB=c++ \
OHOS_DEVICE_TYPE=desktop TAURI_WORKSPACE_ROOT=<tauri-3.0 绝对路径> \
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

target 清理后需重建（空目录即可）。

### 构建后必做：注入 libc++_shared.so（白屏根因）

`libartcraftx_app_lib.so` 因 `BORING_BSSL_RUST_CPPLIB=c++` 带 `DT_NEEDED
libc++_shared.so`，OHOS 不随包提供 → 缺库 = 模块加载失败 = 白屏。注入点必须选
**`gen/ohos/tauri/libs/arm64-v8a/`**（@tauri/app HAR 模块目录，经 HAR 依赖链随 entry 打进
HAP）。**不要**放 `entry_desktop/libs/`（构建被 `ohrs --dist` 同步删除）或 `dialog/libs/`
（被 tauri-cli 重建删除）。首次构建创建 `gen/ohos/tauri` 后才能注入。

### init 后必做的三处手工修正（gen/ 不入库，重建即重打）

1. **ability.har 路径**（`gen/ohos/entry_{desktop,mobile}/oh-package.json5`）：模板生成
   `file:../../../../../../../openharmony-ability/ability.har`——7 级上溯到 `D:\xuqiu` 后
   缺 `tauri-3.0/` 一段，正确值：
   `file:../../../../../../../tauri-3.0/openharmony-ability/ability.har`。
2. **摘除 READ_PASTEBOARD**（两份 `gen/ohos/entry_*/src/main/module.json5` + 可留残串）：
   受限 ACL 权限，默认调试 profile 不放行 → 装机报 `9568289 grant request permissions
   failed`。**根因是本机 `cargo-tauri.exe`（10-12 构建）内嵌的旧模板还带该条目**——fork
   源码模板已改为「deliberately NOT declared」并附 README 指引，装的 CLI 二进制比源码
   新。fork 侧不用动（只消费），init 后摘掉即可。
3. `libc++_shared.so` 注入（见上节）。

## 真机验证

`verify-launch.ps1`（仓根）：`bm uninstall -n` → `hdc install` → `hilog -r` → `aa start` →
pidof → hilog 启动链 → faultlog 检查 → `snapshot_display` 截图。判据（2026-11-03 全过）：

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
