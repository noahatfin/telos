# Telos Multi-Agent Findings（Codex）

- 生成方式：Codex 主代理 + 4 个并行评审代理（架构 / 性能 / 可靠性 / DX）
- 讨论轮次：2 轮（独立评审 + 交叉反驳）
- 日期：2026-02-28
- 范围：`/Users/yingwei/telos`

## 1. 结论总览

多代理结论高度一致：Telos 的方向正确，但当前实现存在“安全边界、数据可信度、可扩展性、契约稳定性”四类核心缺陷。  
其中最紧急的是 **stream 名路径穿越** 与 **对象读取完整性验证缺失**，其次是 **仓库层完整性约束不足** 与 **查询全量扫描/N+1**。

## 2. 共识问题（Consensus）

## 2.1 P0 - 安全边界缺陷：stream 名未做路径约束

现象：
- `stream name` 直接参与路径拼接，缺少路径规范化与目录边界校验。

证据：
- `crates/telos-store/src/refs.rs:31`
- `crates/telos-store/src/refs.rs:69`
- `crates/telos-store/src/refs.rs:95`
- `crates/telos-cli/src/commands/stream.rs:7`

影响：
- 可通过 `../` 或绝对路径绕出 `.telos/refs/streams`，造成越界写/删文件风险。
- 对本地自动化、CI、多人协作环境是 release blocker 级别风险。

建议：
- 同时在 CLI 层和 Store 层做 stream 名校验。
- 禁止绝对路径、`..`、空段、控制字符。
- 对最终路径做 canonicalize，并强制 `starts_with(streams_dir)`。

## 2.2 P1 - 数据可信度缺陷：读取时未校验内容哈希一致性

现象：
- `read` 按路径读取并反序列化对象，但未校验 `hash(bytes) == ObjectId(path)`。

证据：
- `crates/telos-store/src/odb.rs:57`
- `crates/telos-store/src/odb.rs:61`
- `docs/adr/001-content-addressable-storage.md:84`（ADR 声明了完整性要求）

影响：
- 文件被篡改/位腐蚀且仍可反序列化时，会被当作有效对象。
- 破坏 CAS（content-addressable storage）最核心的信任基础。

建议：
- 在读路径增加哈希复算和一致性校验。
- 增加 `IntegrityMismatch` 错误类型。
- 提供 `fsck`/health-check 模式做全库完整性检测。

## 2.3 P1 - 数据丢失可见性缺陷：`iter_all` 静默跳过损坏对象

现象：
- 遍历对象时 `if let Ok(obj) = self.read(&id)`，读失败对象被忽略。

证据：
- `crates/telos-store/src/odb.rs:87`
- `crates/telos-store/src/odb.rs:88`
- `crates/telos-store/src/query.rs:17`
- `crates/telos-store/src/query.rs:52`

影响：
- `query/context` 得到的是“静默缺失”的结果，调用方难判断是“无数据”还是“数据坏了”。

建议：
- 严格模式默认报错，至少在输出中携带 corruption diagnostics。
- 为 CLI 提供 `--strict` 或显式降级策略。

## 2.4 P1 - 约束落点不合理：完整性校验集中在 CLI，Store 层不足

现象：
- CLI 对 `decide` 做了对象类型校验，但 `Repository` 写入未统一兜底。

证据：
- `crates/telos-cli/src/commands/decide.rs:20`
- `crates/telos-cli/src/commands/decide.rs:25`
- `crates/telos-store/src/repository.rs:111`
- `crates/telos-store/src/repository.rs:119`

影响：
- 非 CLI 调用路径可能写入 dangling refs / wrong-type refs。
- 导致语义图谱不可靠，后续分析与自动化判断失真。

建议：
- 把 invariant enforcement 下沉到 `Repository` 边界。
- CLI 校验仅做“提前失败优化”，不作为唯一保障。

## 2.5 P1 - 扩展性缺陷：查询全库扫描 + `context` N+1

现象：
- query 以 `iter_all()` 为基础，`context` 对每个命中 intent 再查一次 decisions。

证据：
- `crates/telos-store/src/query.rs:17`
- `crates/telos-store/src/query.rs:52`
- `crates/telos-store/src/odb.rs:65`
- `crates/telos-cli/src/commands/context.rs:11`
- `crates/telos-cli/src/commands/context.rs:80`

影响：
- `context` 复杂度近似 `O((I+1)*N)`，对象增长时延迟和 IO 成本显著上升。

建议：
- 先把 `context` 改成“一次扫描 decisions 后按 intent_id 分组再 join”。
- 再落地二级索引（`impact -> intent_ids`, `intent_id -> decision_ids`）。

## 2.6 P1 - 并发正确性缺陷：stream tip 更新可能丢写

现象：
- tip 更新是读-改-写，无 CAS（compare-and-swap）语义。

证据：
- `crates/telos-store/src/refs.rs:160`
- `crates/telos-store/src/refs.rs:163`
- `crates/telos-cli/src/commands/intent.rs:12`

影响：
- 并发写入时可能发生 last-writer-wins，导致一部分意图对象从 HEAD 视角不可达。

建议：
- 引入 `expected_tip -> new_tip` CAS 更新语义。
- 冲突时重试/回滚并给出明确错误。

## 2.7 P2 - 外部契约与文档一致性不足

现象：
- 文档示例与当前 CLI 参数/JSON 包装存在偏差。
- 对查询能力描述超前于当前实现（索引未落地）。

证据：
- `README.md:140`
- `README.md:178`
- `README.md:201`
- `docs/INTEGRATION.md:103`
- `crates/telos-cli/src/main.rs:95`
- `crates/telos-cli/src/commands/context.rs:22`

影响：
- 用户与 agent 集成容易踩坑，增加采用成本。

建议：
- 固化 versioned JSON schema，输出 `schema_version`。
- 建立 docs-as-tests，确保 README/INTEGRATION 示例可执行。

## 3. 讨论分歧（Disagreements）

1. 关于优先级：
- 一派认为应先做安全与完整性（路径穿越、哈希验证、静默损坏）。
- 一派认为性能与并发（N+1、CAS、锁竞争）同样应置顶。

2. 关于文档问题权重：
- 一派认为是次级 DX 问题。
- 一派认为这是 adoption 关键阻塞（尤其 AI agent 集成）。

3. 关于“同对象并发写入 race”的严重度：
- 共识为“应修复”，但多数认为其危害更偏可用性/重试体验，而非直接数据腐化。

## 4. 最终收敛优先级（Merged Priority）

1. **Priority 0（立即）**
- 修复 stream 路径穿越（输入验证 + 路径约束）。
- 补齐读取哈希验证与损坏显式上报。

2. **Priority 1（短期）**
- 将引用完整性校验下沉至 Repository。
- 修复并发下 stream tip lost-update（CAS + retry）。

3. **Priority 2（中期）**
- 重构 query/context 执行模型（去 N+1 + 索引化）。
- 提供分页/limit/cursor 语义。

4. **Priority 3（持续）**
- 稳定 CLI/JSON 契约与文档一致性（schema version + docs-as-tests）。

## 5. 建议路线图（建议 6-8 周）

1. Week 1-2：
- 路径安全修复
- 哈希完整性验证
- 损坏对象错误可见化

2. Week 3-4：
- Repository 层 invariant enforcement
- CAS tip 更新与并发冲突处理

3. Week 5-6：
- `context` 去 N+1
- 索引 MVP（impact / intent_id）
- 查询分页参数

4. Week 7-8：
- JSON schema 固化
- 文档命令可执行验证（docs CI）

## 6. 备注

本文件为“多代理讨论收敛稿”，详见讨论纪要：`DEBATE_TRANSCRIPT.md`。  
基线报告见：`CODEX_ANALYSIS_REPORT.md`。

