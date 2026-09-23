# 语言支持（0.3）

RepoTower 根据源文件中的静态关系画图，不需要安装对应语言环境。所有 Tree-sitter 语法解析器随 Rust 分析程序一起提供；分析时不调用 Python、JDK、C/C++ 编译器、Go、Cargo、.NET SDK 或项目构建脚本。

一个节点对应一个已扫描源文件。底层边是「使用者 → 依赖」，界面反过来显示为「依赖 → 使用者」，方便沿箭头查看断开后的潜在影响。图包含类型依赖与条件分支中的引用，因此“受影响”不等于“必定运行失败”。

## Java

文件：`.java`。

- 按 `package` 和源码类型声明建立索引，包括类、接口、枚举、record、注解类型和嵌套类型。
- 解析显式 import、static import，以及能通过同包、显式导入或通配导入确定的类型引用。
- `import p.*` 只扩展名称查找范围；只有源码实际引用的可确定类型才形成文件边。静态导入会依赖所属类型的文件。
- 类型全名存在多个源码候选时报告歧义，不按文件名随意选择。

例如 `Service.java` 中的 `import app.Config;` 连到声明 `app.Config` 的文件；同包代码中的 `Config` 类型也可以建立这条关系，文件名本身不是判断依据。

边界：不运行 Maven/Gradle，不建立真实 classpath，不读取 JAR，不执行注解处理器。只做语法和源码类型索引，不完整推断变量/表达式类型、继承成员或重载调用。把多个构建变体放在同一扫描目录可能产生歧义；可用 `.repotowerignore` 缩小源码集合。

## Python

文件：`.py`、`.pyi`。

- 解析 `import`、`from ... import ...`、别名和多行语句。注释、普通字符串中的 import 文本不会产生边。
- 绝对名称从所选项目根目录及常见 `src/` 布局查找；相对导入按当前包目录解析。
- `import pkg.child` 会依赖路径上实际存在的 `__init__.py`，以及 `child.py`。无初始化文件的 namespace package 可以解析到唯一的真实文件，也支持两个源码根目录中无冲突的 namespace 部分。
- 同名 `.py` 与 `.pyi` 优先选择 `.py`；仅有 stub 时可以连到 `.pyi`。
- `from pkg import name` 先检查包中可识别的声明和导入绑定，避免把普通值误连到同名 `name.py`；没有此类导出时再查子模块。字面量字符串列表形式的 `__all__` 用于通配导入。
- 根目录/src 同名候选、缺失相对目标、无法确定的包成员、动态 `__getattr__` 或非字面量 `__all__` 会给出提示。

例如 `pkg/__init__.py` 定义了 `Settings` 类，那么 `from pkg import Settings` 依赖初始化文件；即使存在 `pkg/Settings.py`，也不会凭名字强行连过去。

边界：不执行包初始化代码，不读取已安装环境，不模拟 sys.path 修改、PYTHONPATH、导入钩子、动态 `importlib` / `__import__` 调用或任意打包布局。条件赋值和动态修改的包导出无法完整推断。标准库和未匹配的绝对导入作为外部依赖，图中不展开。

Python 包初始化与 namespace 语义参照 [Python 导入系统](https://docs.python.org/3/reference/import.html)；这里只实现上述静态子集。

## C / C++

文件：`.c`、`.h`、`.cc`、`.cpp`、`.cxx`、`.hpp`、`.hh`、`.hxx`、`.inl`、`.ipp`。

- 解析 `#include "path"` 和 `#include <path>`。
- 引号路径优先相对当前文件查找。不会通过全项目同名文件或路径后缀猜测目标，项目里的 `mock/stdio.h` 不会替代系统 `stdio.h`。
- 可读取项目根目录的 `compile_commands.json`。按其中当前源文件的 `directory`、`file` 及 `arguments` 或简单带引号的 `command` 提取 `-I`、`-iquote`、`-isystem` 和 MSVC `/I`，只把已扫描的项目内目标连成边；`command` 只分词，绝不执行。
- 引号搜索顺序为当前文件目录、`-iquote`、`-I`/`/I`、`-isystem`；尖括号不使用前两项。同一源文件有多个编译配置时，只有目标一致才连线。外部目录可能遮蔽后面的本地目标时保留未解析提示；忽略或不可读的头文件也不会被后面的同名文件替代。
- 没有明确搜索路径时，本地同名候选记为未解析；未匹配的尖括号路径视为外部头文件，缺失的引号路径记为未解析。
- 宏计算的 include 与 `#include_next` 给出未解析提示。

例如 `src/main.cpp` 包含 `../include/config.hpp`，可以直接建立依赖；对于 `#include <lib/config.hpp>`，编译数据库中明确的 `-Iinclude` 才允许连接到 `include/lib/config.hpp`。

边界：只读取根目录编译数据库，不运行预处理器、不求值 `#if`、不读取外部头文件或系统 SDK。头文件不会继承任意源文件的编译配置；没有自身明确配置时，只确认它的相对引号包含。响应文件、shell 操作、变量展开及额外搜索规则不执行，并保留未解析提示。多个条件分支的 include 会同时保留。当前图是包含关系，不是函数调用或链接符号图；C++20 模块 import 不在这一版范围内。

## Go

文件：`.go`。

- 读取项目内 `go.mod` 的 module 名称，把 import 路径映射到已扫描本地包，支持嵌套 module 边界。
- Go 导入的单位是包，因此一条包导入保守地连接到包内所有已扫描生产文件，不包括 `_test.go`。这不表示每个函数都会被执行。
- 同包无需 import 的名称，利用源码声明与引用索引连接到唯一声明文件；局部同名绑定会抑制不确定的匹配。
- 路径对应多个本地 module、生产文件存在冲突包名、缺失本地包或跨越嵌套 module 边界时报告问题。

例如 module 为 `example.com/app`，`cmd/main.go` 导入 `example.com/app/config`，会依赖 `config/` 下该包的生产源码。标准库及其他未匹配 module 不下载、不展开。

边界：不执行 Go 类型检查，不解析方法接收者或完整词法作用域，不模拟 build tags、GOOS/GOARCH 文件筛选、cgo、go.work、replace、vendor 或 GOPATH 模式。平台变体可能同时出现；同包名称索引是保守近似。导入路径中非常规的十六进制/八进制字符串转义可能记为未解析。

## Rust

文件：`.rs`。

- 从常见 `lib.rs` / `main.rs`、bin/tests/examples 根文件和未被模块树引用的独立文件建立源码模块树。
- 解析 `mod name;`，查找 `name.rs` 或 `name/mod.rs`；支持内联模块中的后续声明，以及字面量 `#[path = "..."]`。
- 解析 `use` 的 `crate` / `self` / `super` 路径、嵌套导入列表、别名及通配路径，也识别表达式和类型中的显式 `crate::...` / `self::...` / `super::...` 路径。文件边指向已声明模块树中能够确定的最长模块前缀。
- 读取附近 Cargo.toml 的 package/lib 名称，识别已扫描本地库名称；没有匹配的外部 crate 不展开。
- 两种模块文件同时存在、模块路径缺失或越界、歧义本地库名给出未解析提示。

例如 `service.rs` 中 `use crate::config::Config;` 依赖声明 config 模块的源码；它不需要真的构建 crate。`lib.rs` 的 `mod config;` 本身也形成模块文件关系。

边界：不运行 Cargo 或 build.rs，不展开宏，不求值 cfg，不完整解析 Cargo workspace/依赖重命名、非标准 manifest target、include! 生成代码和编译器符号查找。use 路径依据模块前缀，不能证明最终成员实际存在。独立文件采用可见源码布局的假设。

## C#

文件：`.cs`。

- 按源码 namespace 和类型声明建立索引；支持块状/文件作用域命名空间、嵌套类型和泛型类型名称。
- `using` 用于名称查找，类型别名和 `using static` 可以直接依赖所属类型；支持源码里的 global using、global:: 和 alias::。普通 namespace using 不会把整个命名空间连到当前文件。
- 可确定的类型引用、类型创建与属性类型等连接到声明文件；partial 类型的引用可能连接到多份声明源码。
- 无法区分的重复声明、冲突类型与缺失本地目标保留诊断，不依赖文件名猜测类型。

例如 `using App.Core;` 本身不产生整个目录的边；当代码使用 `Config` 类型且能唯一对应 `App.Core.Config` 时，才连接到该类型的文件。

边界：不运行 MSBuild/Roslyn，不加载程序集或 NuGet，不完整解析 `.csproj`、目标框架、项目引用、预处理符号、SDK 隐式 global using 与 source generator。显式 global using 按整个所选源码集合应用，建议选择具体项目根目录。源码类型索引不能替代编译器的继承、扩展方法、重载和类型推断；扫描范围包含多个项目时可用忽略规则排除无关源码。

## JavaScript / TypeScript

文件：`.js`、`.jsx`、`.ts`、`.tsx`、`.mjs`、`.cjs`、`.mts`、`.cts`。

- 解析静态 import、export-from、字面量 `import()` / `require()` 和 TypeScript `import = require()`。类型导入也参与影响计算。
- 解析相对路径、准确文件名、扩展名补全、目录 index，以及常见 `.js → .ts/.tsx` 和 `.jsx → .tsx` 回退。
- 动态表达式、越界路径和已知别名前缀给出未解析提示；其他裸包名导入视为外部依赖。

边界：不解释 tsconfig paths/baseUrl、package exports、workspace 包名、require.resolve 或框架虚拟模块；不解析 `.vue` / `.svelte`。同名局部 require 函数可能产生假阳性。这里是文件导入图，不是符号使用或 tree-shaking 结果。

## 共同限制

只扫描 UTF-8 源码，并遵守 `.gitignore`、`.ignore`、`.repotowerignore`。忽略的文件不会重新作为依赖节点读入，符号链接不跟随。外部库和未知动态关系无法完整进入影响计算；缺边会低估影响，保守的条件/包级边可能高估影响。

上限为 10,000 个文件、单文件 2 MiB、合计源码 128 MiB、200,000 个目录项和 100,000 条内部边。超限结果附有提示。界面单次最多绘制 250 个节点，实际影响总数由 Rust 对完整保留图计算。

混合语言项目可以一次扫描；各解析器仍按本语言规则建立关系。JNI、FFI、P/Invoke、插件约定、资源文件与字符串中的跨语言调用不会自动变成文件边。
