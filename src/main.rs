use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Sekaipack v1.0 - Package sekai with resources");
        eprintln!(
            "Usage: {} <main_executable> [resource_dirs...] [-o output]",
            args[0]
        );
        eprintln!(
            "Example: {} test_env/sekai.x86_64 test_env/script test_env/sounds -o example_game",
            args[0]
        );
        std::process::exit(1);
    }

    let main_exe = &args[1];
    let output: String = if args.contains(&"-o".to_string()) {
        let pos = args.iter().position(|x| x == "-o").unwrap();
        args[pos + 1].clone()
    } else {
        "example_game".to_string()
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

    println!("Sekaipack v1.0");
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
    // 启动器源码（编译时嵌入）
    const LAUNCHER_C: &str = include_str!("launcher.c");

    // 临时目录
    let temp_dir = "temp_build";
    fs::create_dir_all(temp_dir)?;

    // 写入启动器源码
    fs::write(format!("{}/launcher.c", temp_dir), LAUNCHER_C)?;

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
