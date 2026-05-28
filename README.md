# MyCapsLock

Windows 轻量级 CapsLock 修饰键工具。**短按**切换大小写，**长按**触发自定义快捷键。

## 使用

1. 将 `mycapslock.exe` 放到任意目录，双击运行
2. 首次运行自动生成 `config.toml`（可自定义键位）
3. 系统托盘出现图标，右键菜单：**Open Config** / **Check Config** / **Auto Start** / **Exit**
4. 勾选 **Auto Start** 即开启开机自启
5. 修改 `config.toml` 保存即生效，无需重启

## 默认快捷键

按住 CapsLock 不放，配合以下键位：

### 光标移动（ESDF）

| 键 | 功能 |
|----|------|
| E / S / D / F | ↑ ← ↓ → |
| W / R | Home / End |

### 选中

| 键 | 功能 |
|----|------|
| I / J / K / L | Shift+↑←↓→（方向选中） |
| U / O | 选到行首 / 行尾 |
| Space | 选中当前行 |

### 删除

| 键 | 功能 |
|----|------|
| N | Backspace（删左字符） |
| M | Delete（删右字符） |
| Y | 删到行首 |
| H | 删到行尾 |
| Backspace | 删除当前行 |

### 换行

| 键 | 功能 |
|----|------|
| Alt+Enter | 在上方插入空行 |
| Enter | 在下方插入空行 |

## 配置文件

编辑 `config.toml`（与 exe 同目录），语法如下：

```toml
[settings]
hold_threshold_ms = 200   # 长按判定时间（毫秒）
tap_to_toggle = true      # 短按是否切换大小写

[mappings.cursor]
# 格式：触发键 = "动作"
e = "Up"                          # 单键
a = "Ctrl+Left"                   # 组合键（+ 连接）
l = "Home, Shift+End, Delete"     # 动作序列（, 分隔）
"alt+e" = "Shift+Up"                # 修饰符前缀（CapsLock+Alt+E）
```

### 键名参考

**触发键（`=` 左边）**：

| 按键 | 键名 |
|------|------|
| 字母 A-Z | `a` ~ `z` |
| 数字 0-9 | `0` ~ `9` |
| `,` `.` `;` `/` `\` `-` `=` `'` `[` `]` | `comma` `period` `semicolon` `slash` `backslash` `minus` `equals` `quote` `bracketleft` `bracketright` |
| 退格 ← | `backspace` |
| Tab ↹ | `tab` |
| 回车 ↵ | `enter` |
| 空格 | `space` |
| Delete | `delete` |
| Home / End | `home` `end` |
| PageUp / PageDown | `pageup` `pagedown` |
| ↑↓←→ | `up` `down` `left` `right` |
| Esc | `escape` |
| F1-F12 | `f1` ~ `f12` |
| 修饰键前缀 | `shift+` `ctrl+` `alt+` `meta+` |

**动作（`=` 右边）**：

| 类型 | 可选值 |
|------|--------|
| 方向键 | `Up` `Down` `Left` `Right` |
| 导航键 | `Home` `End` `PageUp` `PageDown` |
| 编辑键 | `Delete` `Backspace` `Tab` `Enter` `Space` `Escape` |
| 修饰键 | `Ctrl` `Alt` `Shift` `Meta` |
| 功能键 | `F1` ~ `F12` |
| 组合键 | `Ctrl+Left` `Shift+Home` |
| 动作序列 | `Home, Shift+End, Delete`（逗号分隔，顺序执行） |

## 文件清单

| 文件 | 说明 |
|------|------|
| `mycapslock.exe` | 主程序（单文件即可运行） |
| `config.toml` | 用户配置文件（自动生成，可自定义） |
| `default.toml` | 默认配置模板（可选，部署时与 exe 同目录） |
| `mycapslock.log` | 运行日志（自动生成） |

## 构建

```bash
cargo build --release
```

输出：`target/release/mycapslock.exe`

`default.toml` 编译时嵌入 exe，首次运行自动生成 `config.toml`。
