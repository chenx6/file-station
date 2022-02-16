# file-station

## Features

- Don't need nginx, apache, just download a binary and run
- Authentication
- Upload, download, delete and search files
- Rename, move file
- Share files
- Preview audio/video/image/markdown

## Quick start

### Download 

You can get binary from [HERE](https://github.com/chenx6/file-station/releases)

### Add some options

Add environment variable below to custom ui

| Name | Default | Explain |
| - | - | - |
|FS_FOLDER|./files|File folder, store all files in here|
|FS_DATABASE|./database.db|Database position|
|FS_LISTEN|127.0.0.1:5000|Listen host and port|
|FS_REGISTER|TRUE|Can register or not ("TRUE" or "FALSE")|

### Run

```bash
chmod +x ./file-station
./file-station
```

## Alternatives

- [h5ai](https://larsjung.de/h5ai/)
- [zhaojun1998/zfile](https://github.com/zhaojun1998/zfile)
- [files.gallery](https://www.files.gallery/)
- [XnSger/EvoDire](https://github.com/XnSger/EvoDire)
