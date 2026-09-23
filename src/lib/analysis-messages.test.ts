import { describe, expect, it } from 'vitest';
import { translateAnalysisMessage } from './analysis-messages';

describe('analysis message localization', () => {
  it('retains the complete resolver limitation in both translated languages', () => {
    const warning =
      '包名导入视为外部依赖；tsconfig paths、工作区包和 package.json exports 不在本版解析范围。';
    for (const language of ['en', 'ja'] as const) {
      const translated = translateAnalysisMessage(warning, language);
      expect(translated).not.toContain('包名导入');
      expect(translated).toContain('tsconfig paths');
      expect(translated).toContain('package.json exports');
    }
    expect(translateAnalysisMessage(warning, 'zh')).toBe(warning);
  });

  it('preserves Unicode paths, punctuation and OS details without translating inside them', () => {
    const path = 'E:\\团队\\无法读取\\文：件.ts';
    const detail = `${path}：Access denied (os error 5)`;
    expect(translateAnalysisMessage(`无法读取 ${detail}`, 'en')).toBe(`Could not read ${detail}`);
    expect(translateAnalysisMessage(`文件超过 2 MiB，已跳过：${path}`, 'ja')).toBe(
      `2 MiB を超えるファイルをスキップしました: ${path}`,
    );
    expect(translateAnalysisMessage(`${path} 包含解析错误，依赖结果可能不完整。`, 'en')).toContain(
      path,
    );
  });

  it('unwraps Electron errors and retains sidecar exit codes and stderr', () => {
    const wrapped =
      "Error: Error invoking remote method 'repotower:demo': Error: 分析引擎已退出（12）。请重新打开项目。\nE:\\src\\项目: ENOENT";
    expect(translateAnalysisMessage(wrapped, 'en')).toBe(
      'The analysis engine exited (12). Reopen the project.\nE:\\src\\项目: ENOENT',
    );
    expect(translateAnalysisMessage('分析引擎已退出（已中止）。请重新打开项目。', 'ja')).toContain(
      '(中断)',
    );
  });

  it('retains numeric scan bounds, counts and incomplete-result warnings', () => {
    expect(
      translateAnalysisMessage('目录项超过 200000，扫描提前结束；当前结果仅代表已扫描部分。', 'en'),
    ).toBe(
      'Directory entries exceeded 200000. The scan stopped early; results cover only the scanned portion.',
    );
    expect(translateAnalysisMessage('另有 43 条同类警告未显示。', 'ja')).toContain('43');
    expect(
      translateAnalysisMessage(
        '读取的源码达到 128 MiB 安全限制；文件可能在扫描期间发生变化，结果仅代表已读取部分。',
        'en',
      ),
    ).toContain('only the portion read');
  });

  it('preserves unknown diagnostics and distinguishes paths outside the project', () => {
    expect(translateAnalysisMessage('路径位于所选项目之外。', 'en')).toBe(
      'The path is outside the selected project.',
    );
    expect(translateAnalysisMessage('未知状态：src/文件.ts [E42]', 'en')).toBe(
      'Analysis warning: 未知状态：src/文件.ts [E42]',
    );
    expect(translateAnalysisMessage('spawn E:\\engine.exe ENOENT', 'ja')).toBe(
      'spawn E:\\engine.exe ENOENT',
    );
  });

  it('localizes English language diagnostics even when the selected UI language is Chinese', () => {
    const reason = 'Go import crosses a nested module boundary.';
    expect(translateAnalysisMessage(reason, 'zh')).toBe('Go 导入跨越了嵌套模块的边界。');
    expect(translateAnalysisMessage(reason, 'ja')).toBe(
      'Go のインポートがネストしたモジュールの境界を越えています。',
    );
    expect(translateAnalysisMessage(reason, 'en')).toBe(reason);
    const python = 'Python __all__ 不是静态字符串列表；通配导入的子模块可能不完整。';
    expect(translateAnalysisMessage(python, 'en')).toContain(
      'Submodules from wildcard imports may be incomplete',
    );
    expect(translateAnalysisMessage(python, 'ja')).toContain('__all__');
    expect(translateAnalysisMessage(python, 'zh')).toBe(python);
  });

  it('translates new repository limitations while retaining technical configuration names', () => {
    const go =
      'Go 包导入展开为本地包的生产源码文件；不执行 go 命令，也不应用构建标签、平台筛选或 replace 指令。';
    const rust =
      'Rust 分析静态模块声明和引用；不展开宏、不执行构建脚本、不应用 cfg 条件或非标准 Cargo 目标配置。';
    const cpp =
      'C/C++ 分析静态包含和项目根 compile_commands.json 中的显式搜索路径；不运行编译器或预处理器，不继承头文件编译配置，条件分支均保留。无法确定的包含会标为未解析。';
    for (const language of ['en', 'ja'] as const) {
      expect(translateAnalysisMessage(go, language)).toContain('replace');
      expect(translateAnalysisMessage(go, language)).not.toContain('包导入展开');
      expect(translateAnalysisMessage(rust, language)).toContain('cfg');
      expect(translateAnalysisMessage(rust, language)).toContain('Cargo');
      expect(translateAnalysisMessage(rust, language)).not.toContain('不展开宏');
      expect(translateAnalysisMessage(cpp, language)).toContain('compile_commands.json');
      expect(translateAnalysisMessage(cpp, language)).not.toContain('不继承头文件');
    }
  });

  it('unwraps a mixed-language parse failure without translating diagnostic-like filenames', () => {
    const path = 'src/团队：Rust file was not indexed.rs';
    const wrapped = `Error: Error invoking remote method 'repotower:scan': Error: 无法解析 ${path}：Rust file was not indexed`;
    expect(translateAnalysisMessage(wrapped, 'zh')).toBe(`无法解析 ${path}：Rust 源码未建立索引。`);
    expect(translateAnalysisMessage(wrapped, 'en')).toBe(
      `Could not parse ${path}: Rust file was not indexed`,
    );
    expect(translateAnalysisMessage(wrapped, 'ja')).toContain(path);
    for (const language of ['zh', 'en', 'ja'] as const) {
      const unknown = 'E:/src/Go import crosses a nested module boundary./main.go: EACCES';
      expect(translateAnalysisMessage(unknown, language)).toBe(unknown);
    }
  });
});
