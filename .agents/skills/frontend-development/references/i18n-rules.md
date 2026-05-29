# I18n Rules

## Goal

多语言资源要方便人和 AI 就近维护，也要方便脚本精确定位问题。当前只支持中文和英文。

## Locale Contract

- 全仓 canonical locale 固定为 `zh_Hans` 与 `en_US`。
- 前端 App 运行态、UI 资源文件名、用户偏好、API locale、插件 / provider catalog 都使用同一套 canonical locale。
- URL、浏览器语言和 `Accept-Language` 可以接收 `zh-CN`、`zh-Hans`、`en-US`、`en` 等别名，但进入系统后必须归一化为 `zh_Hans` 或 `en_US`。
- 不要为了展示方便改 DTO 字段名或新增 locale 别名字段。

## Placement

- 前端 UI 与插件 / provider locale 文件名统一固定为 `zh_Hans.json` 与 `en_US.json`。
- UI 文案跟随最近 owner：`app-shell/i18n`、`features/*/i18n`、`shared/ui/*/i18n`。
- 中央 i18n 入口只负责发现、注册、校验和加载，不承载全量业务文案。
- 只有跨 feature 且语义稳定的短 UI 词才能进入 common；业务句子不进 common。

## Key And Value Rules

- Key 的每个 JSON 段必须只使用英文小写字母；多个语义单词用 `_` 连接，例如 `primary_action`，不要用驼峰、短横线、数字、中文或空格。
- Key 只要求在 owner 内唯一；跨 owner 同 key 允许存在，专项复盘时可用 `i18n-hygiene --include-cross-owner-warnings` 查看 advisory warning。
- 相同展示 value 在同一 owner、同一 locale 内是 error；优先让调用方复用已有 key，或改成语义更准确的文案。
- 跨 owner 相同 value 允许存在；只有语义完全一致且足够稳定时才上提 common，专项复盘时可用 `--include-cross-owner-warnings` 查看 advisory warning。
- 前端 `i18n/` key 没有静态代码引用是 `unused-i18n-key` warning；保留时必须说明动态 key、路由配置或外部渲染入口原因。
- 中英文文件 key 必须完全对齐；非法 key 命名、缺 key、多 key、JSON 重复 key 都是 error。
- 不要为了消灭重复字符串跨 feature 复用业务 key；错误复用比局部重复更危险。

## QA Evidence

- 多语言资源、key 命名、文案抽取、格式转换或 common 归属变更，必须运行或引用 `node scripts/node/tooling.js i18n-hygiene`。
- 报告产物固定为 `tmp/test-governance/i18n-hygiene.json`。
- 交付时区分 error 与 warning：error 必须修，warning 需要说明是否保留局部语义或动态引用。
- AI 新增文案前应先看同 owner 现有 key；`i18n-hygiene` 的 owner-local duplicate key / value warning 用来辅助复用稳定 key，跨 owner advisory 不作为盲目上提 common 的理由。
