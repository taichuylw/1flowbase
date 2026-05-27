# I18n Hygiene Gate

## Goal

多语言 QA 要给出可复盘证据：哪个 owner、哪个 locale、哪个 key 或 value 违反规则。不要只说“翻译不统一”。

## Command

```bash
node scripts/node/tooling.js i18n-hygiene
```

报告固定写入 `tmp/test-governance/i18n-hygiene.json`。

## Severity

- `error`: 必须修复后才能通过 QA。包括缺 locale 文件、非法 locale 文件名、中英文 key 不对齐、JSON 重复 key、同 owner 同 locale 重复 value。
- `warning`: 需要人工或 AI 复盘。包括跨 owner 重复 key / value；只有语义完全一致且稳定时才建议上提 common。

## Review Rules

- 同 owner 重复 value：优先让调用方复用已有 key，或调整文案使语义更精确。
- 跨 owner 重复 value：默认保留局部 owner；不要为了消灭 warning 抽错 common。
- 新增 common 前先确认它是短 UI 词，不是业务句子。
- QA 报告必须列出命令、报告路径、error 数、warning 数和未修 warning 的保留原因。
