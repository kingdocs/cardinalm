import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignoreFileRules';

// 个人使用的默认排除清单：只为"让结果里不出现垃圾文件"，
// 不影响任何目录的索引（目录永不因文件名规则被排除）。
// - `*.log`、`*.tmp`、`*.bak`、`*.cache`：日志与临时产物
// - `.DS_Store`、`~$*`、`*.swp`：macOS / Office / Vim 的伴随文件
const DEFAULT_IGNORE_FILE_RULES = [
  '*.log',
  '*.tmp',
  '*.bak',
  '*.cache',
  '.DS_Store',
  '~$*',
  '*.swp',
];

const cleanRules = (next: string[]): string[] =>
  next.map((item) => item.trim()).filter((item) => item.length > 0);

export function useIgnoreFileRules() {
  const [ignoreFileRules, setIgnoreFileRulesState] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_IGNORE_FILE_RULES,
    read: (raw) => {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      return cleanRules(parsed.filter((item): item is string => typeof item === 'string'));
    },
    write: (value) => JSON.stringify(value),
    readErrorMessage: 'Unable to read saved ignore file rules',
    writeErrorMessage: 'Unable to persist default ignore file rules',
  });

  const setIgnoreFileRules = useCallback(
    (next: string[]) => {
      setIgnoreFileRulesState(cleanRules(next));
    },
    [setIgnoreFileRulesState],
  );

  return {
    ignoreFileRules,
    setIgnoreFileRules,
    defaultIgnoreFileRules: DEFAULT_IGNORE_FILE_RULES,
  };
}
