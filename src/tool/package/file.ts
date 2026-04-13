//文件类型
interface File {
    name: string;
    size: number;
}
// 文件操作工具包
interface Package {
    /**
     * 读取文件
     * @param path 文件路径
     */
    read(path: string): File;

    /**
     * 写入文件
     * @param path 文件路径
     * @param content 文件内容
     * @return 写入结果
     * @example
     * write('/path/to/file.txt', 'Hello, World!')
    */
    write(path: string, content: string): string;
}
