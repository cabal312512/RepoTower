# Language samples

Open any child folder in RepoTower. Each contains three small source files. Disconnect the config file: the other two should be affected. Every displayed dependency comes from the source; no file needs to be compiled or executed.

| Folder | Language | Disconnect                    |
| ------ | -------- | ----------------------------- |
| java   | Java     | src/main/java/toy/Config.java |
| python | Python   | config.py                     |
| c      | C        | config.h                      |
| cpp    | C++      | config.hpp                    |
| go     | Go       | config/config.go              |
| rust   | Rust     | src/config.rs                 |
| csharp | C#       | Config.cs                     |

Rust also declares its modules in main.rs, so main.rs has a direct dependency on config.rs. The graph reflects that declaration as well as the service's use of config. Other examples form a three-file chain.

The whole language-samples folder can also be opened as one mixed repository. These independent examples are not intended to link across language boundaries. Samples are synthetic MIT-licensed RepoTower source.
