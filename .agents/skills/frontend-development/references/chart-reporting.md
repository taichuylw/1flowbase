# Chart / Reporting

命中报表、图表、`ECharts` 宿主渲染或低代码 JS Block chart primitive 时读取本文件。

## Target

报表 / 图表能力默认以 `echarts` 作为宿主渲染依赖；低代码 JS Block 只拿到受控 `Chart / EChart` primitive / facade，不直接拿真实 React 组件、DOM、`echarts` 实例或任意 npm 包。

## Budget

第一版只开放可 JSON 校验的 `option`、尺寸、主题 token 和声明式事件桥接；不开放 formatter 函数、custom series、任意 HTML tooltip、外部图片、地图资源、raw instance 或用户侧 resize / dispose。

## Placement

`echarts` 依赖和内部渲染组件归 `@1flowbase/block-renderer` 或明确可信 feature owner；不要新增 `echarts-for-react` / 其它 wrapper，除非先说明维护收益、安全影响和替代验证。

## Evidence

新增 Chart primitive 时必须同步 `page-protocol` primitive / schema 校验、`antd-facade` factory、`block-renderer` 渲染与单元测试；涉及页面展示再补目标页面或 style-boundary 证据。

## Stop Condition

一旦需求需要用户函数、外部资源、跨区块共享实例、地图扩展或直接暴露 ECharts API，停止实现并回到 `problem-framing` 做 contract / 安全边界决策。
