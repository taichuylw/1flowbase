import { useRef } from 'react';

let stableListItemKeySeed = 0;

function createStableListItemKey(prefix: string) {
  stableListItemKeySeed += 1;
  return `${prefix}-${stableListItemKeySeed}`;
}

export function useStableListItemKeys(prefix: string, length: number) {
  const keysRef = useRef<string[]>([]);

  while (keysRef.current.length < length) {
    keysRef.current.push(createStableListItemKey(prefix));
  }
  if (keysRef.current.length > length) {
    keysRef.current.length = length;
  }

  function insertItemKey(index = keysRef.current.length) {
    keysRef.current.splice(index, 0, createStableListItemKey(prefix));
  }

  function removeItemKey(index: number) {
    keysRef.current.splice(index, 1);
  }

  function moveItemKey(from: number, to: number) {
    if (to < 0 || to >= keysRef.current.length) {
      return;
    }

    const [itemKey] = keysRef.current.splice(from, 1);
    if (!itemKey) {
      return;
    }

    keysRef.current.splice(to, 0, itemKey);
  }

  return {
    itemKeys: keysRef.current,
    insertItemKey,
    moveItemKey,
    removeItemKey
  };
}
