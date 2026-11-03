//! 桌面专属 `WebviewWindowBuilder` 方法在 mobile/OHOS 编译面的空操作垫片（OHOS 穿刺用）。
//!
//! Tauri 的 `fullscreen` / `always_on_top` / `center` 等 builder 方法带 `#[cfg(desktop)]`，
//! mobile/ohos 编译面上不存在（artcraftx 原本只编过桌面面）。本 trait 在该面提供
//! 同名空操作：调用点全部传默认值语义（`fullscreen(false)` / `always_on_top(false)` /
//! 无参 `center()`），让渡无行为影响。desktop 编译面本 trait 无任何方法，调用点自动
//! 回落到固有方法，零行为影响。

use tauri::{Manager, Runtime, WebviewWindowBuilder};

pub trait DesktopOnlyBuilderCompat: Sized {
  #[cfg(any(mobile, target_env = "ohos"))]
  fn fullscreen(self, fullscreen: bool) -> Self {
    let _ = fullscreen;
    self
  }

  #[cfg(any(mobile, target_env = "ohos"))]
  fn always_on_top(self, always_on_top: bool) -> Self {
    let _ = always_on_top;
    self
  }

  #[cfg(any(mobile, target_env = "ohos"))]
  fn center(self) -> Self {
    self
  }
}

impl<'a, R: Runtime, M: Manager<R>> DesktopOnlyBuilderCompat for WebviewWindowBuilder<'a, R, M> {}
