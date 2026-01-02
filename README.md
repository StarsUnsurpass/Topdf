# Topdf - 通用文档转 PDF 工具

**项目作者:** StarsUnsurpass ([主页](https://github.com/StarsUnsurpass))
**项目地址:** [https://github.com/StarsUnsurpass/Topdf](https://github.com/StarsUnsurpass/Topdf)

Topdf 是一款基于 Rust 开发的高性能、跨平台文档转换工具。它拥有现代化的图形界面，支持将多种常见格式的文件一键转换为 PDF。

## 主要特性

*   **多格式支持:**
    *   **文档:** Microsoft Word (`.docx`), 纯文本 (`.txt`)
    *   **数据:** JSON, XML, CSV
    *   **网页/标记:** Markdown (`.md`), HTML
    *   **图片:** PNG, JPG, JPEG, BMP
    *   **代码:** Rust, Python, JavaScript, C, C++
*   **中文支持:** 内置智能字体加载策略，优先适配系统中文环境（如微软雅黑、SimHei、DroidSansFallback），解决 PDF 中文乱码问题。
*   **批量处理:** 支持一次性添加多个文件进行批量转换，内置多线程并行处理，速度极快。
*   **美观界面:** 基于 `iced` 框架打造的现代化暗色主题界面，操作简单直观。
*   **进度实时反馈:** 清晰的进度条和状态指示，让您随时掌握转换进度。

## 更新日志

### [v0.2.0] - 2026-01-02
- **新增格式支持**: 增加对 YAML, TOML 配置文件以及 Excel (.xlsx, .xls) 表格文件的转换支持。
- **依赖更新**: 引入 `serde_yaml`, `toml` 和 `calamine` 用于处理新格式。
- **UI 增强**: 完善了文件过滤器及功能说明。

## 如何运行

### 前置条件
确保您的系统中已安装 Rust 编程环境 (Cargo)。

### 编译与运行
1.  **构建发布版本:**
    ```bash
    cargo build --release
    ```
2.  **运行程序:**
    ```bash
    cargo run --release
    ```
    或者直接运行编译好的可执行文件:
    ```bash
    ./target/release/Topdf
    ```

## 使用说明

1.  **添加文件:** 点击左上角的 **“+ 添加文件”** 按钮，选择您需要转换的文件；或者直接将文件 **拖拽** 到程序窗口的文件列表区域。
2.  **选择输出目录 (可选):** 默认情况下，生成的 PDF 文件会保存在源文件相同的目录下。如果您希望保存到其他位置，请点击 **“选择输出文件夹”** 按钮进行设置。
3.  **开始转换:** 点击右下角的 **“开始转换”** 绿色按钮。
4.  **查看结果:** 程序将自动开始处理，并在列表中实时显示每个文件的转换状态（成功/失败）。

## 常见问题

*   **中文乱码:** 如果转换出的 PDF 中文显示为方框，请确保您的系统安装了常见的中文字体（如 Windows 的“微软雅黑”/“黑体”，Linux 的 `DroidSansFallback` 或 `NotoSansCJK`）。
*   **转换失败:** 某些复杂的 DOCX 格式可能无法完美还原，建议先另存为简单的文档格式。

---
*Built with StarsUnsurpass.*
