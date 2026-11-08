// build.js
import {build} from 'vite'
import minimist from 'minimist'
import * as fs from "node:fs";

// npm run build-render -- --name=Grant --path=Grant
//npm run build-render -- --all
interface Arg {
    //输出名
    name?: string;
    //组件包路径
    path?: string;
    all?: boolean;
}


async function runBuild(arg: Arg) {
    const renderPath = arg.path;

    if (!renderPath) {
        console.error('❌ 请指定组件路径: --path=xxx/ComponentA');
        return
    }
    console.log(`📦 正在编译组件 ${renderPath}...`);
    process.env.name = arg.name || renderPath;
    process.env.path = `render/${renderPath}`;
    await build();
    console.log(`✅ 组件 ${renderPath} 编译完成！`);
}

// 获取命令行参数: npm run build-render --name=ComponentA
const args = minimist(process.argv.slice(2)) as Arg;

(async () => {
    if (args.all) {
        //读取src/render目录下第一层目录
        const renderDir = fs.readdirSync('src/render');
        for (let dir of renderDir) {
            const arg = {
                name: dir,
                path: dir
            }
            await runBuild(arg);
        }
        return
    }
    await runBuild(args);
})()
