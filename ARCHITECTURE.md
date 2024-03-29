# Architecture

## Design

### 目标

本程序为单用户模式，并且可以分享链接，文件链接可以带加密密钥。

数据库存储着用户名，密码。分享的文件夹，url，密钥，每次获取时查询。

user 表:

|名字|类型|说明|
| - | - | - |
|用户名|VARCHAR(32)||
|密码|VARCHAR(32)|存入哈希，使用 `argon2` 算法来保护密码|

share 表：

|名字|类型|说明|
| - | - | - |
|path|VARCHAR|相对路径（每个路径只有 1 个分享链接）|
|url|VARCHAR||
|password|VARCHAR|密码（可为空）|

### 逻辑

文件列表可以点击文件下载，点击目录进入，点击菜单显示 Modal 框进行更多操作。

获取分享时查询文件夹/文件是否被分享，如果发现需要密码但是没有提供就显示弹窗要求输入密码。

多选时可以删除，下载，移动文件。

### Endpoint

API 使用 RESTful 设计。入口对应的函数可以从 "main.rs" 中的 `main` 函数中找到。

- `/api/v1/`
  - `/auth`
    - `POST` Login
  - `/users`
    - `POST` Register
  - `/user`
    - `PATCH` Modify password
  - `/file`
    - `GET, DELETE, PATCH, POST` File resource
  - `/files`
    - `GET, POST` Folder resource
  - `/search`
    - `GET` Search file/folder
  - `/share`
    - `POST, GET, DELETE` Share file/folder resource
  - `/shares`
    - `GET` Get all share folders
  - `/assets`
    - SPA Assets
