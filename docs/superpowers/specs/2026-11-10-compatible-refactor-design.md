# EK86317A Programmer 兼容式重构设计

日期：2026-11-10  
基线：上游 `main` 的 `0d3fe0dbf03ae3855fe279ebcf713b9accb8a89f`，版本 `0.5.1`。

## 目标与边界

在保留现有 React + TypeScript + Rust/Tauri 2 技术栈和全部界面操作逻辑的前提下，降低状态耦合、保证硬件操作串行、消除重复协议调度，并建立无硬件测试基线。

不改变：
- 页面、标签、按钮文案与主要视觉布局。
- 芯片选择、扫描、连接、Detect、固件导入、DAC/EEPROM 读写、Verify ALL、BIN 导出、UART 的使用顺序。
- 现有 Tauri 命令名称和参数协议。
- 原生桥接器的总线寻址、寄存器地址、延时和 EEPROM 控制命令。
- 面板名清理规则、文件名规则、现有 localStorage 键。

本次不做：技术栈整体迁移、硬件 UI 改版、提交或推送 Git。

## 方案比较

1. **保留 Rust/Tauri 的兼容式重构（采用）**：原生桥接及芯片协议直接复用；构建轻，改动可局部验证。
2. **Qt/C++ 重写**：单一原生语言，但需要移植驱动绑定、UART 和全部 React 页面，风险高。
3. **Go/Electron 或 Go/WebView**：跨平台容易，但原生桥接、系统 WebView 和应用打包的复杂度更高，内存收益不确定。

## 架构

### 前端

- `context/`：领域独立的 React Context 和 Provider。
  - 连接状态：设备枚举、连接/断开、Detect。
  - 寄存器状态：DAC、EEPROM、批量替换和单寄存器更新。
  - 日志状态：日志独立更新，避免寄存器页重渲染。
  - 操作状态：统一硬件操作队列及 busy 状态。
- `hooks/useAppContext.ts`：维持旧 API 的兼容组合入口，组件逐步迁移到领域 hooks。
- `lib/register-store.ts`：无副作用的 Map 更新函数，值未改变时保留原引用。
- `lib/operation-queue.ts`：Promise 队列；单个任务失败不阻断后续任务；挂载后清空状态；已卸载组件不再更新状态。
- `lib/pmu-operations.ts`：GAMMA 等批量寄存器操作；保持高低字节顺序。
- 现有组件保留视觉 DOM 结构，改用领域 hooks。

### Rust 后端

- `pmu/device.rs` 保持桥接 trait 与芯片设备核心。
- 将寄存器读取上下文、批量 GAMMA 调度提取到明确的服务边界，保持逐寄存器 I²C 语义和地址顺序。
- `bridges/ftdi/` 分离开发 Mock 与原生 FTDI 驱动；不改变 MPSSE 实现。
- 先运行基线测试，再逐层移动实现，避免同时修改协议和结构。

### 测试与交付

- Vitest：寄存器状态、队列、芯片换算、GAMMA 调度。
- Rust `cargo test`：现有寄存器、UART、固件与 Mock 测试全部保留，并补关键服务测试。
- 前端生产构建与 Rust 原生构建实际执行。
- 浏览器中使用 Tauri Mock IPC 验证 Scan → Connect → Detect → Read/Write/Export 的 UI 状态，不伪造真实硬件通过结论。
- 无真实芯片和桥接器时，明确区分 Mock/软件验证与待真机验收项。

## 回滚

所有改动保留在本地工作树。以原始源码 ZIP 和基线提交哈希为回滚依据；不覆盖上游分支、不自动提交。按文件反向恢复即可，原生 vendor SDK 不做变更。
