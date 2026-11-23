[English](README.md) · [Русский](README.ru.md) · [Français](README.fr.md) · [Español](README.es.md) · [العربية](README.ar.md) · [中文](README.zh.md)

# erlu

**面向桌面和浏览器的 IDEF0 功能建模编辑器，使用 Rust 和 egui 构建。**

erlu 帮助您描述系统的功能及其输入、控制、输出和机制。从上下文图开始，通过逐层分解逐步细化模型。

本程序面向使用 IDEF0 分析和设计流程的分析师、工程师及架构师。开发者可以通过 WASM 插件扩展编辑器，并连接外部工具和 AI 智能体。

## 功能

- **IDEF0 建模：**功能块、正交箭头、分支、分解、继承接口、隧道和功能引用。
- **文档级历史记录：**在图之间切换后仍可撤销和重做，涵盖界面、AI 和插件产生的修改。
- **模型校验：**检查结构、连接、编号和 IDEF0 语义规则。
- **成本分析：**设置功能成本，并按分解层级汇总。
- **导出：**PDF、SVG 和 Markdown。
- **外观：**浅色与深色主题、对象颜色；应用界面支持英语、俄语、法语、西班牙语、阿拉伯语和简体中文。
- **自动化：**桌面端 MCP，以及浏览器中的外部智能体连接器。

内置检查有助于发现建模错误，但不等同于完整的标准符合性认证。请参阅[校验范围](docs/validation.md)。

## 桌面与 Web

桌面应用面向 Linux、Windows 和 macOS。浏览器版本使用相同的模型、历史记录、校验和导出引擎。

文档在本地处理。在浏览器中编辑无需账户或文档数据库服务器。通过文件选择器打开文件，通过下载保存文件。应用提供 IndexedDB 恢复副本，并在安装应用缓存后支持离线使用。恢复副本不能代替保存文件。

Linux/NixOS 及 Chromium、Firefox 和 WebKit 已经过本地测试。其他平台和浏览器引擎的状态见[验收记录](docs/acceptance.md)。

## 插件

内置三个 WASM 插件：

| 插件 | 用途 |
| --- | --- |
| Colors | 设置选中功能块和箭头的颜色 |
| [Responsibility Map](plugins/responsibility_map/README.md) | 组织角色、职责分配、检查及 RACI 矩阵导出 |
| [Semantic Dictionary](plugins/semantic_dictionary/README.md) | 概念与同义词、箭头绑定、使用位置搜索和标签一致性 |

同一个 WASM 组件在桌面端通过 Wasmtime 运行，在浏览器端通过 Worker 运行。插件可以添加命令、检查、导出器和输入表单。

插件修改经过共享事务引擎，并支持撤销和重做。首版 SDK 不提供新建模语言注册或任意自定义界面控件。

## 从源码快速启动

### Nix / NixOS

在仓库根目录运行：

```sh
nix-shell
npm --prefix web ci
npm --prefix web run plugins
```

以下命令分别用于启动桌面应用、打开示例模型，以及在 `http://127.0.0.1:8080` 启动 Web 应用：

```sh
cargo run -p erlu
```

```sh
cargo run -p erlu -- examples/order-fulfilment.idef0
```

```sh
trunk serve
```

Nix shell 提供构建工具和运行库，无需再通过 Cargo 安装 Trunk。按编译器区分的构建目录可防止 Nix 与 rustup 产物混用。

### 不使用 Nix

需要 Rust、Node.js 22+、Python 3、Trunk 和 wasm-tools。Rust 版本固定在 `rust-toolchain.toml` 中。

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --version 1.263.0 --locked
cargo install trunk --version 0.21.14 --locked
npm --prefix web ci
npm --prefix web run plugins
```

然后使用上方的启动命令。Linux 还需要 X11/Wayland 库、libxkbcommon 和 OpenGL。

## 基本操作

| 操作 | 控制方式 |
| --- | --- |
| 添加功能块 | 鼠标右键 |
| 创建箭头 | 从端口或图边界拖动 |
| 创建分支 | 按住 Ctrl 从箭头拖动 |
| 编辑属性 | 双击 |
| 打开分解图 | 在功能块上单击鼠标中键 |
| 返回上一层 | 在背景上单击鼠标中键 |
| 撤销 / 重做 | Ctrl+Z / Ctrl+Shift+Z |
| 适应窗口 | F |

macOS 上的常用编辑快捷键使用 Cmd。

## AI 与自动化

将 MCP 客户端连接到独立编辑器进程，或控制已打开桌面窗口中的文档：

```sh
cargo run -p erlu -- mcp --headless
```

```sh
cargo run -p erlu -- --mcp
```

连接信息位于 **AI** 菜单中。MCP 修改与用户操作共享历史记录和校验；保存到磁盘需要显式执行。

在浏览器中，外部智能体通过 [JavaScript/TypeScript SDK](web/sdk/erlu-agent.js) 连接。用户为指定来源启用访问，并可随时撤销。智能体适配器由集成方提供，无需内置 AI 聊天。参阅 [AI API](docs/ai.md)。

## 开发者信息

| Crate | 职责 |
| --- | --- |
| `erlu-core` | 模型、命令、事务、历史记录、校验、几何与导出 |
| `erlu-app` | egui 界面、桌面/Web 适配器和 MCP |
| `erlu-plugins` | 桌面端 WASM 组件加载与执行 |

界面、MCP、浏览器连接器和插件共用同一操作注册表。修订号检查用于检测异步编辑中的冲突。

文档采用带版本号和显式能力列表的 JSON `.idef0` 格式。迁移注册表及对未知可选数据的保留机制支持未来兼容性。不支持旧 Qt 文档和 Qt 二进制插件。

当前支持的建模语言是 IDEF0。DFD、其他 IDEF 系列和 BPMN 尚未实现为编辑模式。

### 检查与构建

```sh
cargo test --workspace --locked
cargo build -p erlu --release --locked
trunk build --release --locked
npm --prefix web test
```

静态 Web 构建产物位于 `dist/web`，可通过 HTTPS 托管。[发布工作流](.github/workflows/release.yml) 构建桌面压缩包、Web ZIP 和校验和。该工作流面向 GitHub Actions；Forgejo 需要适配 CI。发布工作流尚未在远程运行。

## 文档

- [架构](docs/architecture.md)
- [界面本地化](docs/localization.md)
- [浏览器测试与 CI](docs/browser-tests.md)
- [文档格式与兼容性](docs/format.md)
- [AI API 与 MCP](docs/ai.md)
- [插件 SDK](docs/plugins.md)
- [IDEF0 校验](docs/validation.md)
- [验收记录](docs/acceptance.md)

## 许可证

项目代码采用 LGPL-3.0-or-later 许可证。附带字体的许可证见 [assets/fonts/LICENSE-DejaVu.txt](assets/fonts/LICENSE-DejaVu.txt)。

[Noto Sans SC — SIL Open Font License](assets/fonts/LICENSE-NotoSansSC.txt)
