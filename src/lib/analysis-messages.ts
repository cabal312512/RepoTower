import { translateLanguageMessage } from './language-messages';

type Language = 'zh' | 'en' | 'ja';
type Translation = { en: string; ja: string };

// The analyzer's wire format contains Chinese and English diagnostics. Keep this
// adapter at the presentation boundary; never translate identifiers or file paths.
const messages: Record<string, Translation> = {
  '包名导入视为外部依赖；tsconfig paths、工作区包和 package.json exports 不在本版解析范围。': {
    en: 'Package imports are treated as external dependencies. tsconfig paths, workspace packages and package.json exports are not resolved in this version.',
    ja: 'パッケージ名によるインポートは外部依存として扱います。このバージョンでは tsconfig paths、ワークスペースのパッケージ、package.json exports の解決には対応していません。',
  },
  '达到安全限制（10,000 个模块或 128 MiB 源码），结果仅代表已扫描部分。': {
    en: 'The limit of 10,000 modules or 128 MiB of source was reached. Results cover only the scanned portion.',
    ja: '10,000 モジュールまたはソースコード 128 MiB の上限に達しました。結果はスキャン済みの部分のみを対象とします。',
  },
  '读取的源码达到 128 MiB 安全限制；文件可能在扫描期间发生变化，结果仅代表已读取部分。': {
    en: 'The 128 MiB source limit was reached while reading. Files may have changed during the scan; results cover only the portion read.',
    ja: '読み込み中にソースコード 128 MiB の上限に達しました。スキャン中にファイルが変わった可能性があります。結果は読み込み済みの部分のみを対象とします。',
  },
  '内部依赖边超过 100,000，后续边已省略；指标和影响结果可能低估。': {
    en: 'Internal dependencies exceeded 100,000 edges. Further edges were omitted, so metrics and impact may be underestimated.',
    ja: '内部依存が 100,000 辺を超えたため、それ以降の辺を省略しました。指標や影響範囲が実際より小さく表示される可能性があります。',
  },
  '没有找到可分析的 JS/TS 模块。支持 .js、.jsx、.ts、.tsx、.mjs 和 .cjs。': {
    en: 'No JS/TS modules found. Supported extensions: .js, .jsx, .ts, .tsx, .mjs and .cjs.',
    ja: '解析できる JS/TS モジュールが見つかりません。.js、.jsx、.ts、.tsx、.mjs、.cjs に対応しています。',
  },
  '动态表达式无法静态确定目标；未执行项目代码。': {
    en: 'The target of this dynamic expression cannot be determined statically. Project code was not executed.',
    ja: '動的な式の参照先は静的解析では特定できません。プロジェクトのコードは実行していません。',
  },
  '在已扫描文件中未找到模块；它可能不存在、被忽略或使用尚不支持的解析规则。': {
    en: 'Module not found among scanned files. It may be missing, ignored, or use unsupported resolution rules.',
    ja: 'スキャン済みのファイルにモジュールが見つかりません。存在しない、除外されている、または未対応の解決規則を使っている可能性があります。',
  },
  '空模块路径。': { en: 'Empty module path.', ja: 'モジュールのパスが空です。' },
  '尚不解析绝对路径或项目路径别名。': {
    en: 'Absolute paths and project path aliases are not supported yet.',
    ja: '絶対パスとプロジェクトのパスエイリアスにはまだ対応していません。',
  },
  '不支持的相对路径格式。': {
    en: 'Unsupported relative path format.',
    ja: '未対応の相対パス形式です。',
  },
  '带查询参数、片段、空字符或反斜杠的导入未解析。': {
    en: 'Imports containing query parameters, fragments, null characters or backslashes are not resolved.',
    ja: 'クエリパラメーター、フラグメント、ヌル文字、バックスラッシュを含むインポートは解決できません。',
  },
  '路径位于所选项目之外。': {
    en: 'The path is outside the selected project.',
    ja: '選択したプロジェクトの外にあるパスです。',
  },
  '已跳过非 Unicode 文件路径。': {
    en: 'A non-Unicode file path was skipped.',
    ja: 'Unicode ではないファイルパスをスキップしました。',
  },
  '无法计算项目相对路径。': {
    en: 'Could not determine the project-relative path.',
    ja: 'プロジェクトからの相対パスを取得できませんでした。',
  },
  '请选择存在且可读取的项目文件夹。': {
    en: 'Select an existing, readable project folder.',
    ja: '存在し、読み取り可能なプロジェクトフォルダーを選択してください。',
  },
  '解析被中止。': { en: 'Parsing was interrupted.', ja: '構文解析が中断されました。' },
  '未找到该模块，请重新分析项目。': {
    en: 'Module not found. Analyze the project again.',
    ja: 'モジュールが見つかりません。プロジェクトを再解析してください。',
  },
  '该模块已被移除，请选择仍在塔中的模块。': {
    en: 'This module has already been removed. Select a standing module.',
    ja: 'このモジュールはすでに取り除かれています。塔に残っているモジュールを選択してください。',
  },
  '依赖图不一致，请重新分析项目。': {
    en: 'The dependency graph is inconsistent. Analyze the project again.',
    ja: '依存グラフに不整合があります。プロジェクトを再解析してください。',
  },
  '请求超过大小限制。': {
    en: 'The request exceeds the size limit.',
    ja: 'リクエストがサイズの上限を超えています。',
  },
  '缺少项目路径。': {
    en: 'The project path is missing.',
    ja: 'プロジェクトのパスが指定されていません。',
  },
  '请先分析项目并指定模块。': {
    en: 'Analyze a project and select a module first.',
    ja: '先にプロジェクトを解析してモジュールを選択してください。',
  },
  '未知命令。支持 analyze 和 impact。': {
    en: 'Unknown command. Supported commands: analyze and impact.',
    ja: '不明なコマンドです。analyze と impact に対応しています。',
  },
  '分析引擎未找到。开发环境请先运行 scripts/build.ps1，或使用完整的便携发布目录。': {
    en: 'Analysis engine not found. Run scripts/build.ps1 for a development build, or use the complete portable release folder.',
    ja: '解析エンジンが見つかりません。開発環境では scripts/build.ps1 を実行するか、ポータブル版のフォルダー全体を使用してください。',
  },
  '分析结果超过安全大小限制。请选择较小的项目目录。': {
    en: 'The analysis result exceeds the size limit. Select a smaller project folder.',
    ja: '解析結果がサイズの上限を超えています。より小さいプロジェクトフォルダーを選択してください。',
  },
  '分析引擎返回了无法读取的数据。请重新打开项目。': {
    en: 'The analysis engine returned unreadable data. Reopen the project.',
    ja: '解析エンジンから読み取れないデータが返されました。プロジェクトを開き直してください。',
  },
  '分析失败，请重新选择项目。': {
    en: 'Analysis failed. Select the project again.',
    ja: '解析に失敗しました。プロジェクトを選択し直してください。',
  },
  '分析引擎连接已关闭，请重新打开项目。': {
    en: 'The analysis engine connection closed. Reopen the project.',
    ja: '解析エンジンとの接続が閉じられました。プロジェクトを開き直してください。',
  },
  '分析超时，请选择较小的项目目录后重试。': {
    en: 'Analysis timed out. Try a smaller project folder.',
    ja: '解析がタイムアウトしました。より小さいプロジェクトフォルダーでお試しください。',
  },
  '无法发送分析请求，请重新打开项目。': {
    en: 'Could not send the analysis request. Reopen the project.',
    ja: '解析リクエストを送信できませんでした。プロジェクトを開き直してください。',
  },
  '请选择有效的项目文件夹。': {
    en: 'Select a valid project folder.',
    ja: '有効なプロジェクトフォルダーを選択してください。',
  },
  '无法读取此文件夹，请检查路径与读取权限。': {
    en: 'Could not read this folder. Check its path and read permissions.',
    ja: 'フォルダーを読み取れません。パスと読み取り権限を確認してください。',
  },
  '模块参数无效，请重新打开项目。': {
    en: 'Invalid module parameters. Reopen the project.',
    ja: 'モジュールのパラメーターが無効です。プロジェクトを開き直してください。',
  },
  '应用正在关闭。': { en: 'The application is closing.', ja: 'アプリケーションを終了しています。' },
};

const prefixes: Array<[string, Translation]> = [
  [
    'Python 解析器初始化失败：',
    { en: 'Could not initialize the Python parser: ', ja: 'Python の解析器を初期化できません: ' },
  ],
  [
    '无法读取目录项：',
    { en: 'Could not read a directory entry: ', ja: 'ディレクトリエントリを読み取れません: ' },
  ],
  ['忽略规则错误：', { en: 'Ignore rule error: ', ja: '除外ルールのエラー: ' }],
  [
    '为避免越界和循环，已跳过符号链接：',
    {
      en: 'Symlink skipped to avoid leaving the project or following a cycle: ',
      ja: 'プロジェクト外への参照や循環を避けるため、シンボリックリンクをスキップしました: ',
    },
  ],
  [
    '无法读取文件属性：',
    { en: 'Could not read file metadata: ', ja: 'ファイル属性を読み取れません: ' },
  ],
  [
    '文件超过 2 MiB，已跳过：',
    {
      en: 'File exceeds 2 MiB and was skipped: ',
      ja: '2 MiB を超えるファイルをスキップしました: ',
    },
  ],
  [
    '无法打开项目文件夹：',
    { en: 'Could not open the project folder: ', ja: 'プロジェクトフォルダーを開けません: ' },
  ],
  [
    '文件扫描后变大，已跳过：',
    {
      en: 'File grew after scanning and was skipped: ',
      ja: 'スキャン後にサイズが増えたファイルをスキップしました: ',
    },
  ],
  [
    '文件不是 UTF-8，已跳过：',
    { en: 'File is not UTF-8 and was skipped: ', ja: 'UTF-8 ではないファイルをスキップしました: ' },
  ],
  ['无法读取 ', { en: 'Could not read ', ja: '読み取りに失敗しました: ' }],
  ['无法解析 ', { en: 'Could not parse ', ja: '構文解析に失敗しました: ' }],
  ['无效请求：', { en: 'Invalid request: ', ja: '無効なリクエスト: ' }],
  [
    '无法启动分析引擎：',
    { en: 'Could not start the analysis engine: ', ja: '解析エンジンを起動できません: ' },
  ],
  [
    '界面文件无法读取。请使用完整的发布目录或先运行构建。',
    {
      en: 'Could not read the interface files. Use the complete release folder or build the application first.',
      ja: '画面のファイルを読み取れません。リリースフォルダー全体を使用するか、先にアプリケーションをビルドしてください。',
    },
  ],
];

export function translateAnalysisMessage(message: string, language: Language): string {
  // Electron adds this transport wrapper to rejected invoke() promises. The
  // underlying diagnostic, including OS errors and paths, is retained in full.
  const content = message
    .replace(/^(?:Error: )?Error invoking remote method '[^']+': (?:Error: )?/, '')
    .replace(/^Error: /, '');
  const translated = translateLanguageMessage(content, language);
  if (translated !== undefined) return translated;
  // Parse failures can wrap a fixed English diagnostic after a Unicode path.
  // Split only the final separator; a path may itself contain Chinese colons.
  const parseFailure = /^无法解析 ([\s\S]+)：([^：]+)$/.exec(content);
  if (parseFailure) {
    const reason = translateLanguageMessage(parseFailure[2], language);
    if (reason !== undefined)
      return language === 'zh'
        ? `无法解析 ${parseFailure[1]}：${reason}`
        : language === 'en'
          ? `Could not parse ${parseFailure[1]}: ${reason}`
          : `${parseFailure[1]} の構文解析に失敗しました: ${reason}`;
  }
  if (language === 'zh' || !content) return content;
  if (Object.hasOwn(messages, content)) return messages[content][language];

  const suppressed = /^另有 (\d+) 条同类警告未显示。$/.exec(content);
  if (suppressed)
    return language === 'en'
      ? `${suppressed[1]} additional warnings of this kind were omitted.`
      : `同種の警告をさらに ${suppressed[1]} 件省略しました。`;
  const entries = /^目录项超过 ([\d,]+)，扫描提前结束；当前结果仅代表已扫描部分。$/.exec(content);
  if (entries)
    return language === 'en'
      ? `Directory entries exceeded ${entries[1]}. The scan stopped early; results cover only the scanned portion.`
      : `ディレクトリエントリが ${entries[1]} 件を超えたため、スキャンを途中で終了しました。結果はスキャン済みの部分のみを対象とします。`;
  const parseWarning = /^([\s\S]+) 包含解析错误，依赖结果可能不完整。$/.exec(content);
  if (parseWarning)
    return language === 'en'
      ? `${parseWarning[1]} contains parse errors; dependency results may be incomplete.`
      : `${parseWarning[1]} に構文解析エラーがあります。依存関係の結果が不完全な可能性があります。`;
  const interrupted = /^无法解析 ([\s\S]+)：解析被中止。$/.exec(content);
  if (interrupted)
    return language === 'en'
      ? `Could not parse ${interrupted[1]}: parsing was interrupted.`
      : `${interrupted[1]} の構文解析が中断されました。`;
  const exited = /^分析引擎已退出（([^）]+)）。请重新打开项目。([\s\S]*)$/.exec(content);
  if (exited) {
    const code = exited[1] === '已中止' ? (language === 'en' ? 'stopped' : '中断') : exited[1];
    return (
      (language === 'en'
        ? `The analysis engine exited (${code}). Reopen the project.`
        : `解析エンジンが終了しました (${code})。プロジェクトを開き直してください。`) + exited[2]
    );
  }
  for (const [prefix, translated] of prefixes) {
    if (content.startsWith(prefix)) return translated[language] + content.slice(prefix.length);
  }
  // Unknown diagnostics remain available for troubleshooting. Technical English
  // from the OS/parser is already useful; never guess at a filename's language.
  if (!/\p{Script=Han}/u.test(content)) return content;
  return `${language === 'en' ? 'Analysis warning: ' : '解析メッセージ: '}${content}`;
}
