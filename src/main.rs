use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("AppBinder v1.0 - Package applications with resources");
        eprintln!(
            "Usage: {} <main_executable> [resource_dirs...] [-o output]",
            args[0]
        );
        eprintln!(
            "Example: {} test_env/sekai.x86_64 test_env/script test_env/sounds -o bundled_sekai",
            args[0]
        );
        std::process::exit(1);
    }

    let main_exe = &args[1];
    let output: String = if args.contains(&"-o".to_string()) {
        let pos = args.iter().position(|x| x == "-o").unwrap();
        args[pos + 1].clone()
    } else {
        "bundled_app".to_string()
    };
    let mut resource_dirs = Vec::new();

    // 解析参数
    let mut i = 2;
    while i < args.len() {
        if args[i] == "-o" {
            i += 2;
        } else {
            resource_dirs.push(&args[i]);
            i += 1;
        }
    }

    println!("AppBinder v1.0");
    println!("Packaging: {} -> {}", main_exe, output);

    // 检查主程序是否存在
    if !Path::new(main_exe).exists() {
        eprintln!("Error: Main executable '{}' not found", main_exe);
        std::process::exit(1);
    }

    // 开始打包
    match create_bundled_app(main_exe, &resource_dirs, &output) {
        Ok(()) => {
            println!("Successfully created: {}", output);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn create_bundled_app(
    main_exe: &str,
    resource_dirs: &[&String],
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建启动器代码
    let launcher_c = generate_launcher_c();

    // 临时目录
    let temp_dir = "temp_build";
    fs::create_dir_all(temp_dir)?;

    // 写入启动器源码
    fs::write(format!("{}/launcher.c", temp_dir), launcher_c)?;

    // 编译启动器
    println!("Compiling launcher...");
    let output = Command::new("gcc")
        .args(&[
            "-o",
            &format!("{}/launcher", temp_dir),
            &format!("{}/launcher.c", temp_dir),
            "-lz",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!("GCC compilation failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to compile launcher".into());
    }

    // 创建资源包
    println!("Creating resource package...");
    let resource_file = format!("{}/resources.tar.gz", temp_dir);
    create_resource_package(main_exe, resource_dirs, &resource_file)?;

    // 读取启动器和资源
    let launcher_binary = fs::read(&format!("{}/launcher", temp_dir))?;
    let resource_data = fs::read(&resource_file)?;

    // 创建最终的可执行文件
    println!("Creating final executable...");
    {
        use std::io::Write;
        let mut final_exe = fs::File::create(output_file)?;

        // 写入启动器
        final_exe.write_all(&launcher_binary)?;

        // 记录资源偏移
        let resource_offset = launcher_binary.len();

        // 写入资源数据
        final_exe.write_all(&resource_data)?;

        // 写入偏移信息（8字节）
        final_exe.write_all(&(resource_offset as u64).to_le_bytes())?;
    }

    // 设置执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(output_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(output_file, perms)?;
    }

    // 清理临时文件
    fs::remove_dir_all(temp_dir)?;

    Ok(())
}

fn create_resource_package(
    main_exe: &str,
    resource_dirs: &[&String],
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // use std::process::Stdio;

    // 创建临时目录结构
    let temp_structure = "temp_structure";
    fs::create_dir_all(temp_structure)?;

    // 复制主程序
    let _main_path = Path::new(main_exe);
    fs::copy(main_exe, format!("{}/sekai.x86_64", temp_structure))?;

    // 复制资源目录
    for dir in resource_dirs {
        let dir_path = Path::new(dir);
        if dir_path.exists() && dir_path.is_dir() {
            let output = Command::new("cp")
                .args(&["-r", dir, &format!("{}/", temp_structure)])
                .output()?;
            if !output.status.success() {
                return Err(format!("Failed to copy directory: {}", dir).into());
            }
        }
    }

    // 创建tar.gz包
    let output = Command::new("tar")
        .args(&["-czf", output_file, "-C", temp_structure, "."])
        .output()?;

    if !output.status.success() {
        eprintln!("Tar creation failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to create resource package".into());
    }

    // 清理临时目录
    fs::remove_dir_all(temp_structure)?;

    Ok(())
}

fn generate_launcher_c() -> String {
    r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <libgen.h>
#include <stdint.h>

int main(int argc, char *argv[]) {
    if (argc > 1 && strcmp(argv[1], "--version") == 0) {
        printf("bundled app v1.0\n");
        return 0;
    }
    
    // 获取自身路径
    char exe_path[4096];
    ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
    if (len == -1) {
        perror("Failed to get executable path");
        return 1;
    }
    exe_path[len] = '\0';
    
    // 创建临时目录
    char temp_template[] = "/tmp/bundled_app_XXXXXX";
    char *temp_dir = mkdtemp(temp_template);
    if (!temp_dir) {
        perror("Failed to create temp directory");
        return 1;
    }
    
    // 打开自身文件
    int exe_fd = open(exe_path, O_RDONLY);
    if (exe_fd == -1) {
        perror("Failed to open executable");
        return 1;
    }
    
    // 获取文件大小
    struct stat st;
    if (fstat(exe_fd, &st) == -1) {
        perror("Failed to get file size");
        close(exe_fd);
        return 1;
    }
    off_t file_size = st.st_size;
    
    // 读取资源偏移（最后8字节）
    uint64_t offset;
    if (lseek(exe_fd, file_size - 8, SEEK_SET) == -1) {
        perror("Failed to seek to offset");
        close(exe_fd);
        return 1;
    }
    if (read(exe_fd, &offset, 8) != 8) {
        perror("Failed to read offset");
        close(exe_fd);
        return 1;
    }
    
    // 提取资源数据
    if (lseek(exe_fd, offset, SEEK_SET) == -1) {
        perror("Failed to seek to resources");
        close(exe_fd);
        return 1;
    }
    
    char resources_path[512];
    snprintf(resources_path, sizeof(resources_path), "%s/resources.tar.gz", temp_dir);
    
    int resources_fd = open(resources_path, O_CREAT | O_WRONLY, 0644);
    if (resources_fd == -1) {
        perror("Failed to create resources file");
        close(exe_fd);
        return 1;
    }
    
    char buffer[4096];
    ssize_t bytes_read;
    off_t remaining = file_size - 8 - offset;
    while (remaining > 0 && (bytes_read = read(exe_fd, buffer, sizeof(buffer))) > 0) {
        if (bytes_read > remaining) bytes_read = remaining;
        write(resources_fd, buffer, bytes_read);
        remaining -= bytes_read;
    }
    
    close(exe_fd);
    close(resources_fd);
    
    // 解压资源
    char extract_cmd[1024];
    snprintf(extract_cmd, sizeof(extract_cmd), "cd '%s' && tar -xzf resources.tar.gz", temp_dir);
    int result = system(extract_cmd);
    if (result != 0) {
        fprintf(stderr, "Failed to extract resources\n");
        return 1;
    }
    unlink(resources_path);
    
    // 构建主程序路径
    char sekai_path[512];
    snprintf(sekai_path, sizeof(sekai_path), "%s/sekai.x86_64", temp_dir);
    
    // 设置执行权限
    chmod(sekai_path, 0755);
    
    // 准备参数
    char path_arg[512];
    snprintf(path_arg, sizeof(path_arg), "--path=%s", temp_dir);
    
    // 执行主程序
    char *exec_args[argc + 3];
    exec_args[0] = sekai_path;
    exec_args[1] = path_arg;
    
    int j = 2;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--version") != 0) {
            exec_args[j++] = argv[i];
        }
    }
    exec_args[j] = NULL;
    
    execv(sekai_path, exec_args);
    
    // 如果execv返回，说明出错了
    perror("Failed to execute main program");
    
    // 清理临时目录
    char cleanup_cmd[512];
    snprintf(cleanup_cmd, sizeof(cleanup_cmd), "rm -rf '%s'", temp_dir);
    system(cleanup_cmd);
    
    return 1;
}"#
    .to_string()
}
