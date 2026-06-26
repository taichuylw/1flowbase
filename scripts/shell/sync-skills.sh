#!/usr/bin/env bash
# sync-skills.sh - 同步项目 skills 到全局 .claude/skills
#
# Usage:
#   ./.agents/sync-skills.sh [--dry-run] [--verbose]
#
# Options:
#   --dry-run    显示将要执行的操作但不实际执行
#   --verbose    显示详细输出

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 默认选项
DRY_RUN=false
VERBOSE=false

# 解析参数
while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [--dry-run] [--verbose]"
      echo ""
      echo "Options:"
      echo "  --dry-run    显示将要执行的操作但不实际执行"
      echo "  --verbose    显示详细输出"
      exit 0
      ;;
    *)
      echo -e "${RED}错误: 未知参数 '$1'${NC}"
      exit 1
      ;;
  esac
done

# 项目和目标路径
PROJECT_SKILLS="$(cd "$(dirname "${BASH_SOURCE[0]}")/skills" && pwd)"
CLAUDE_SKILLS="${HOME}/.claude/skills"

# 检查源目录
if [[ ! -d "$PROJECT_SKILLS" ]]; then
  echo -e "${RED}错误: 项目 skills 目录不存在: ${PROJECT_SKILLS}${NC}"
  exit 1
fi

# 检查目标目录
if [[ ! -d "$CLAUDE_SKILLS" ]]; then
  echo -e "${YELLOW}警告: 全局 skills 目录不存在，将创建: ${CLAUDE_SKILLS}${NC}"
  if [[ "$DRY_RUN" == false ]]; then
    mkdir -p "$CLAUDE_SKILLS"
  fi
fi

echo -e "${BLUE}=== 同步项目 Skills 到全局 .claude/skills ===${NC}"
echo -e "${BLUE}源目录:${NC} $PROJECT_SKILLS"
echo -e "${BLUE}目标目录:${NC} $CLAUDE_SKILLS"
echo ""

if [[ "$DRY_RUN" == true ]]; then
  echo -e "${YELLOW}[DRY RUN 模式 - 不会实际执行操作]${NC}"
  echo ""
fi

# 统计变量
SYNCED=0
SKIPPED=0
UPDATED=0

# 遍历项目 skills
for skill_dir in "$PROJECT_SKILLS"/*; do
  if [[ ! -d "$skill_dir" ]]; then
    continue
  fi

  skill_name=$(basename "$skill_dir")
  target_dir="$CLAUDE_SKILLS/$skill_name"

  # 检查是否存在 SKILL.md
  if [[ ! -f "$skill_dir/SKILL.md" ]]; then
    if [[ "$VERBOSE" == true ]]; then
      echo -e "${YELLOW}⚠ 跳过 ${skill_name}: 缺少 SKILL.md${NC}"
    fi
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  # 判断是更新还是新增
  if [[ -d "$target_dir" ]]; then
    echo -e "${BLUE}↻ 更新:${NC} $skill_name"
    ACTION="更新"
    UPDATED=$((UPDATED + 1))
  else
    echo -e "${GREEN}+ 新增:${NC} $skill_name"
    ACTION="新增"
    SYNCED=$((SYNCED + 1))
  fi

  # 执行同步
  if [[ "$DRY_RUN" == false ]]; then
    # 使用 rsync 同步（保留权限和时间戳）
    rsync -a --delete "$skill_dir/" "$target_dir/"

    if [[ "$VERBOSE" == true ]]; then
      echo "  └─ 已同步到: $target_dir"
    fi
  else
    if [[ "$VERBOSE" == true ]]; then
      echo "  └─ 将同步到: $target_dir"
    fi
  fi
done

echo ""
echo -e "${BLUE}=== 同步完成 ===${NC}"
echo -e "${GREEN}新增:${NC} $SYNCED 个 skill"
echo -e "${BLUE}更新:${NC} $UPDATED 个 skill"
if [[ $SKIPPED -gt 0 ]]; then
  echo -e "${YELLOW}跳过:${NC} $SKIPPED 个 skill"
fi

if [[ "$DRY_RUN" == true ]]; then
  echo ""
  echo -e "${YELLOW}这是 dry-run 模式，没有实际执行操作${NC}"
  echo -e "${YELLOW}移除 --dry-run 参数以实际执行同步${NC}"
fi
