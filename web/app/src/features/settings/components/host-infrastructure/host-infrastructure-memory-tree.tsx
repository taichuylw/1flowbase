import type { Key, ReactNode } from 'react';

import { Typography } from 'antd';
import type { DataNode } from 'antd/es/tree';

import type { SettingsHostInfrastructureMemoryTreeNode } from '../../api/host-infrastructure';

export type MemoryTreeDataNode = DataNode & {
  inspectionPath: string[];
  label: string;
  parentKey?: Key;
  children?: MemoryTreeDataNode[];
};

type MemoryTreeSearchItem = {
  key: Key;
  parentKey?: Key;
  title: string;
};

export function findTreeKeyByPath(
  nodes: MemoryTreeDataNode[],
  inspectionPath: string[] | null
): string | null {
  if (!inspectionPath) {
    return null;
  }
  const requestedPath = inspectionPath.join('\u001f');
  for (const node of nodes) {
    if (node.inspectionPath.join('\u001f') === requestedPath) {
      return String(node.key);
    }
    const childKey = findTreeKeyByPath(node.children ?? [], inspectionPath);
    if (childKey) {
      return childKey;
    }
  }
  return null;
}

export function toTreeData(
  nodes: SettingsHostInfrastructureMemoryTreeNode[],
  loadedChildren: Record<string, SettingsHostInfrastructureMemoryTreeNode[]>,
  searchValue: string,
  parentKey?: Key
): MemoryTreeDataNode[] {
  return nodes.map((node) => ({
    key: node.node_ref,
    label: node.label,
    parentKey,
    title: renderTreeTitle(node.label, searchValue),
    isLeaf: !node.has_children,
    inspectionPath: node.inspection_path,
    children: loadedChildren[node.node_ref]
      ? toTreeData(
          loadedChildren[node.node_ref],
          loadedChildren,
          searchValue,
          node.node_ref
        )
      : undefined
  }));
}

function renderTreeTitle(label: string, searchValue: string): ReactNode {
  const trimmedSearchValue = searchValue.trim();
  const index = trimmedSearchValue
    ? label.toLowerCase().indexOf(trimmedSearchValue.toLowerCase())
    : -1;
  const labelNode =
    index > -1 ? (
      <span>
        {label.slice(0, index)}
        <span className="host-memory-panel__tree-search-value">
          {label.slice(index, index + trimmedSearchValue.length)}
        </span>
        {label.slice(index + trimmedSearchValue.length)}
      </span>
    ) : (
      <span>{label}</span>
    );

  return (
    <span className="host-memory-panel__tree-node-title">
      <Typography.Text>{labelNode}</Typography.Text>
    </span>
  );
}

export function collectTreeSearchItems(
  nodes: MemoryTreeDataNode[],
  items: MemoryTreeSearchItem[] = []
): MemoryTreeSearchItem[] {
  for (const node of nodes) {
    items.push({
      key: node.key,
      parentKey: node.parentKey,
      title: node.label
    });
    collectTreeSearchItems(node.children ?? [], items);
  }
  return items;
}
