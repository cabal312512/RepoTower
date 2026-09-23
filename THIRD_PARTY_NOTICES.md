# Third-party software

RepoTower source is MIT licensed. Dependencies retain their own licenses; the lockfiles pin the exact versions used by this build.

| Component                                 | Upstream                                                                                                      | License                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Electron                                  | https://github.com/electron/electron                                                                          | MIT; Chromium and other components have their own notices |
| React / React DOM                         | https://github.com/facebook/react                                                                             | MIT                                                       |
| Zustand                                   | https://github.com/pmndrs/zustand                                                                             | MIT                                                       |
| Lucide                                    | https://github.com/lucide-icons/lucide                                                                        | ISC                                                       |
| Tree-sitter runtime and language bindings | https://github.com/tree-sitter/tree-sitter                                                                    | MIT                                                       |
| Java grammar                              | https://github.com/tree-sitter/tree-sitter-java                                                               | MIT                                                       |
| Python grammar                            | https://github.com/tree-sitter/tree-sitter-python                                                             | MIT                                                       |
| C and C++ grammars                        | https://github.com/tree-sitter/tree-sitter-c / https://github.com/tree-sitter/tree-sitter-cpp                 | MIT                                                       |
| Go grammar                                | https://github.com/tree-sitter/tree-sitter-go                                                                 | MIT                                                       |
| Rust grammar                              | https://github.com/tree-sitter/tree-sitter-rust                                                               | MIT                                                       |
| C# grammar                                | https://github.com/tree-sitter/tree-sitter-c-sharp                                                            | MIT                                                       |
| JavaScript and TypeScript grammars        | https://github.com/tree-sitter/tree-sitter-javascript / https://github.com/tree-sitter/tree-sitter-typescript | MIT                                                       |
| petgraph                                  | https://github.com/petgraph/petgraph                                                                          | MIT / Apache-2.0                                          |
| serde / serde_json                        | https://github.com/serde-rs                                                                                   | MIT / Apache-2.0                                          |
| ignore                                    | https://github.com/BurntSushi/ripgrep                                                                         | MIT / Unlicense                                           |

The portable release retains Electron's license as LICENSE.electron.txt where supplied beside the executable, along with LICENSES.chromium.html. Bundled application dependencies, including the compiled language grammars and the Windows launcher's zip, sha2 and crc32fast crates, are covered by the accompanying generated THIRD_PARTY_LICENSES.txt, collected from local installed package and Cargo sources. Development-only tools (Rust, w64devkit, npm packages) stay in ignored project folders and are not bundled into the app. No Python runtime, JDK, .NET SDK or Go toolchain is required or distributed for analyzing those languages.

The bundled demonstration source in fixtures/demo-project is part of RepoTower and uses the same MIT license. It is synthetic example code for visualizing architecture, not an external user's repository.
