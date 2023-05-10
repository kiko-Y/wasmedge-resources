1. 在主机上新建一个文件夹, 用于和容器共享, 设为 /Users/bytedance/dev/wasm

2. 通过官网安装docker, 启动docker.app, 下载镜像wasmedge/wasmedge:ubuntu-build-clang

3. 使用命令 `docker run -it --name wasi-dev --mount type=bind,source=/Users/bytedance/dev/wasm,target=/root/wasm/ wasmedge/wasmedge:ubuntu-build-clang` 启动一个容器. 执行完命令后, 就在容器中开了一个 bash. 容器中的 `/root/wasm/` 就是我们的共享目录.

4. 打开VSCode, 安装Docker扩展, 如图操作
   
   <img title="" src="README.assets/2023-05-10-13-20-15-image.png" alt="" width="560">

5. 此时会打开一个新的VSCode窗口, 左下角会显示你在容器里, 然后打开文件夹即可开始工作.
   
   <img title="" src="README.assets/screenshot-20230510-132245.png" alt="" width="546">

6. 如果想切换文件夹, 该如何操作? 使用 Command + Shift + P 调出命令栏, 输入 File: Open Folder 即可.

本次开发完后, 容器怎么处理?

默认情况下, docker 不会把容器删除, 除非新建容器时指定了某些参数. 当退出容器时, 容器的状态会变成 Exited.

使用 docker container list -a 查看所有容器, 会发现自己上次用的容器还在.

然后用 `docker exec -it wasi-dev /bin/bash` 启动, 其中 wasi-dev 是容器的名字.

Ref

1. https://yeasy.gitbook.io/docker_practice/, 介绍了Docker常用的命令
