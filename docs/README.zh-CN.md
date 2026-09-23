# RepoTower

一个离线的桌面依赖分析小工具。选中文件，模拟断开，沿真实连线观察哪些文件可能受到影响。

[下载最新版本](https://github.com/cabal312512/RepoTower/releases/latest) · [English](../README.md) · [语言支持与限制](LANGUAGES.md)

![RepoTower 浅色依赖图](images/release-light.png)

## 下载与使用

| 平台                   | 文件                                       |
| ---------------------- | ------------------------------------------ |
| Windows x64 便携文件夹 | `RepoTower-0.5.0-windows-x64-portable.zip` |
| Windows x64 单文件下载 | `RepoTower-0.5.0-windows-x64.exe`          |
| macOS Apple Silicon    | `RepoTower-0.5.0-macos-arm64.zip`          |
| macOS Intel            | `RepoTower-0.5.0-macos-x64.zip`            |
| Linux x64              | `RepoTower-0.5.0-linux-x64.tar.gz`         |

便携压缩包需要完整解压再运行。单文件 EXE 首次启动会在旁边创建 `RepoTower-data`，缓存运行文件并保存设置；它是一个文件下载，不是运行后仍只占一个文件。普通文件夹版把设置保存在程序旁的 `runtime-data`。请放在可写目录里。

使用时不需要安装 Node、Rust、Python、JDK、Go 或 .NET SDK，不需要网络。新用户默认英文、白天模式；外观菜单可以切换中文、English、日本語和深浅色主题，已有设置会保留。

发布页附带 SHA-256 校验和、源码 ZIP 和源码 tar.gz。当前程序没有代码签名，macOS 版本没有公证。详见[下载说明](DISTRIBUTION.md)。

## 操作

1. 打开或拖入项目文件夹，也可以点击「试试示例」。
2. 单击文件查看关系，双击进入「相邻」。选择不会重置缩放。
3. 点击「断开文件」，在「影响路径」中按真实依赖距离观察传播。
4. 暂停、单步或重播；重新选中任何直接断开的文件都可以「恢复文件」。

「全部」中按住鼠标右键拖动文件，线和箭头跟随移动。位置菜单和右键菜单可以分别还原一个文件或全部文件的位置。拖动空白处平移，滚轮缩放，`R` 适应画布，`Esc` 取消选择，`Ctrl/Cmd+Z` 撤销。右上角 `?` 查看操作帮助。

支持 **Java、Python、C、C++、Go、Rust、C#、JavaScript、TypeScript**。解析器内置，支持范围与无法确定的引用见[语言说明](LANGUAGES.md)。

**断开只是只读模拟，不会修改、删除或执行任何源文件。** 箭头从依赖指向使用它的文件；“受影响”表示静态依赖上的潜在影响，不代表一定发生运行故障。类型导入也参与计算。

示例有 23 个文件、24 条导入边。断开 `src/core/config.ts` 影响其他 13 个文件，9 个文件仍正常。多次断开后，可通过「影响路径」的下拉框切换记录；逐个恢复会重新计算剩余影响。

## 从源码构建

需要 Node.js 24+、Rust 1.90+ 和本机 C 编译器。首次下载依赖需要网络，完成后的应用离线运行。Windows 可通过 `scripts/build.ps1` 使用项目内的便携工具链，详见[构建说明](PORTABLE-TOOLCHAIN.md)。

```sh
npm ci
node scripts/build-native.mjs
node scripts/distribute.mjs
```

Windows 还可运行 `node scripts/build-single.mjs` 生成单文件下载版。成品和校验和在 `release/artifacts/`。构建工具、依赖、缓存和运行数据不会提交到 Git。

[贡献说明](../CONTRIBUTING.md) · [架构](ARCHITECTURE.md) · [验证记录](VALIDATION.md) · [更新日志](../CHANGELOG.md)

## 许可证

[MIT](../LICENSE)，Copyright © 2026 Cabal and RepoTower contributors。第三方依赖保留各自许可证。
