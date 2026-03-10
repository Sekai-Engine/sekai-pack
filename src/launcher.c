#include <stdio.h>
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
    
    char exe_path[4096];
    ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
    if (len == -1) {
        perror("Failed to get executable path");
        return 1;
    }
    exe_path[len] = '\0';
    
    char temp_template[] = "/tmp/bundled_app_XXXXXX";
    char *temp_dir = mkdtemp(temp_template);
    if (!temp_dir) {
        perror("Failed to create temp directory");
        return 1;
    }
    
    int exe_fd = open(exe_path, O_RDONLY);
    if (exe_fd == -1) {
        perror("Failed to open executable");
        return 1;
    }
    
    struct stat st;
    if (fstat(exe_fd, &st) == -1) {
        perror("Failed to get file size");
        close(exe_fd);
        return 1;
    }
    off_t file_size = st.st_size;
    
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
    
    char extract_cmd[1024];
    snprintf(extract_cmd, sizeof(extract_cmd), "cd '%s' && tar -xzf resources.tar.gz", temp_dir);
    int result = system(extract_cmd);
    if (result != 0) {
        fprintf(stderr, "Failed to extract resources\n");
        return 1;
    }
    unlink(resources_path);
    
    char sekai_path[512];
    snprintf(sekai_path, sizeof(sekai_path), "%s/sekai.x86_64", temp_dir);
    //printf("%s", temp_dir);
    fflush(stdout);

    int devnull = open("/dev/null", O_WRONLY);
    dup2(devnull, STDOUT_FILENO);
    dup2(devnull, STDERR_FILENO);
    close(devnull);

    chmod(sekai_path, 0755);
    
    char *exec_args[argc + 4];
    exec_args[0] = sekai_path;
    exec_args[1] = "--path";
    exec_args[2] = temp_dir;
    
    int j = 3;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--version") != 0) {
            exec_args[j++] = argv[i];
        }
    }
    exec_args[j] = NULL;
    
    execv(sekai_path, exec_args);
    
    perror("Failed to execute main program");
    
    char cleanup_cmd[512];
    snprintf(cleanup_cmd, sizeof(cleanup_cmd), "rm -rf '%s'", temp_dir);
    system(cleanup_cmd);
    
    return 1;
}
