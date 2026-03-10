# Sekaipack

Sekaipack 是Sekai引擎的主要打包工具，用于将主程序及其资源文件打包成单个可执行文件。打包后的可执行文件在运行时会自动解压资源到`/tmp`临时目录并执行程序。

## 平台支持

因时间限制，支持平台功能会在后续逐步完善.

- 支持 Linux 系统
- 需预装 gcc （大部分linux发行版默认自带）

## 编译

```bash
cargo build --release
```

编译完成后，可执行程序位于 `target/release/sekaipack`。

## 使用方法

```bash
./sekaipack <主程序> [资源目录...] [-o 输出文件名]
```

### 示例

```bash
# 打包主程序和资源目录为test
./sekaipack test_env/sekai.x86_64 test_env/script test_env/sounds -o example_gamae

# 使用默认输出文件名
./sekaipack test_env/sekai.x86_64 test_env/resources
```

### 参数说明

- `<主程序>`: sekai模板文件路径
- `[资源目录...]`: 要打包的资源目录（可选，多个目录用空格分隔）
- `-o 输出文件名`: 指定输出文件名称（默认为`example_game`）

